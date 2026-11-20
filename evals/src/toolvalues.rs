//! Value-bearing tool evidence (accepted decision 0119 §4, amending 0075 §4).
//!
//! The structural record (`tool-calls.json`) names calls by digest and never
//! by value. This module keeps the *argument values* of the two
//! harness-owned tools — Swarmail's MCP tools and the fixture's `eval_http` —
//! and nothing else, under four rules:
//!
//! * **Redacted at the source.** Values are captured during the trial while
//!   the harness still holds its secrets, and masked there against a sealed
//!   registry: the trial's fixture admin bearer and every literal the
//!   operator registered as a sentinel. Only the redacted, checked form is
//!   ever written; original bytes live in bounded process memory only.
//! * **Fail-closed, whole-trial.** One call that cannot be proven redacted
//!   withholds the trial's whole value record with exactly one typed
//!   [`Refusal`]. There is never a prefix, a placeholder or an empty file.
//! * **Bounded and hash-bound.** Each value, each trial and each published
//!   file has a byte cap; each published entry carries the structural
//!   record's ordinal, lengths and digests, and the companion is bound to the
//!   structural file it extends. The journal's digest of the original bytes
//!   must equal the capture's, so an entry the journal does not corroborate
//!   is a `binding_mismatch`, never a guess.
//! * **Never scored.** Scoring reads the fixture state alone (0055); nothing
//!   here is an input to a verdict, a row or an accounting figure.
//!
//! Model text and reasoning are never retained (0075 §1): a transcript here is
//! the ordered tool-call sequence with redacted arguments, the outcome class
//! and the row, and cannot explain an agent's private rationale.

use crate::artifacts::{self, Binding};
use crate::journal::{self, Fact, ToolCall};
use crate::provenance::{is_hash, sha256};
use crate::runner::TrialResult;
use crate::state::FailureClass;
use crate::tasks::Outcome;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Arc, Mutex};

/// One original or redacted argument's cap, inclusive.
pub const VALUE_LIMIT: usize = 8192;
/// The most harness-owned calls one trial may make and keep values.
pub const CALL_LIMIT: usize = 256;
/// One trial's original argument bytes, inclusive.
pub const TRIAL_LIMIT: usize = 65536;
/// One published companion's cap, inclusive.
pub const COMPANION_LIMIT: usize = 65536;
/// The sealed registry's caps: bytes over all literals and distinct literals.
pub const REGISTRY_BYTES: usize = 65536;
pub const REGISTRY_LITERALS: usize = 256;
/// The deepest JSON nesting a value may have.
pub const DEPTH_LIMIT: usize = 32;
/// One transcript leaf's cap and the transcript index's cap, inclusive.
pub const LEAF_LIMIT: usize = 128 * 1024;
pub const INDEX_LIMIT: usize = 64 * 1024;
/// Transcripts kept per failure class.
pub const PER_CLASS: usize = 3;
/// What a registered literal is replaced by.
pub const MASK: &str = "[REDACTED]";
/// The capture's private intermediate file names.
pub const CAPTURE_FILE: &str = "capture.json";
pub const MANIFEST_FILE: &str = "manifest.json";
/// The private capture record's own cap.
pub const CAPTURE_LIMIT: usize = 4 * COMPANION_LIMIT;
/// The companion's and the transcripts' published names.
pub const COMPANION_FILE: &str = "tool-values.json";
pub const TRANSCRIPTS_DIR: &str = "transcripts";
pub const TRANSCRIPT_INDEX: &str = "index.json";
/// The environment variable that names the trial's private observation
/// socket to the `eval_http` helper. It carries a path, never a secret.
pub const ENV_CAPTURE_SOCKET: &str = "EVAL_TOOL_CAPTURE_SOCKET";

/// Why a present tool-call fact has no value record. A closed set, in the
/// order the report checks them: the first that holds is the one reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Refusal {
    CaptureMissing,
    RegistryUnavailable,
    UnknownTool,
    ValueTooLarge,
    TrialTooLarge,
    InvalidArgumentEncoding,
    ResidualSecret,
    BindingMismatch,
    EvidenceWriteFailed,
}

impl Refusal {
    pub const ALL: [Refusal; 9] = [
        Refusal::CaptureMissing,
        Refusal::RegistryUnavailable,
        Refusal::UnknownTool,
        Refusal::ValueTooLarge,
        Refusal::TrialTooLarge,
        Refusal::InvalidArgumentEncoding,
        Refusal::ResidualSecret,
        Refusal::BindingMismatch,
        Refusal::EvidenceWriteFailed,
    ];
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CaptureMissing => "capture_missing",
            Self::RegistryUnavailable => "registry_unavailable",
            Self::UnknownTool => "unknown_tool",
            Self::ValueTooLarge => "value_too_large",
            Self::TrialTooLarge => "trial_too_large",
            Self::InvalidArgumentEncoding => "invalid_argument_encoding",
            Self::ResidualSecret => "residual_secret",
            Self::BindingMismatch => "binding_mismatch",
            Self::EvidenceWriteFailed => "evidence_write_failed",
        }
    }
}

// ---- A strict JSON reader that keeps byte spans -------------------------

/// A JSON value as written: numbers keep their spelling, objects their
/// member order. Duplicate member names, depth over [`DEPTH_LIMIT`],
/// non-UTF-8 bytes and lone surrogate escapes are refused.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(String),
    Str(String),
    Arr(Vec<Node>),
    Obj(Vec<(String, Node)>),
}

/// A value and the byte range it occupies in its input.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    pub value: Json,
    pub span: std::ops::Range<usize>,
}

impl Node {
    pub fn get(&self, key: &str) -> Option<&Node> {
        match &self.value {
            Json::Obj(members) => members.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }
    fn as_str(&self) -> Option<&str> {
        match &self.value {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
}

struct Reader<'a> {
    b: &'a [u8],
    i: usize,
}

impl Reader<'_> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }
    fn eat(&mut self, c: u8) -> Result<(), ()> {
        if self.b.get(self.i) == Some(&c) {
            self.i += 1;
            Ok(())
        } else {
            Err(())
        }
    }
    fn word(&mut self, w: &[u8]) -> Result<(), ()> {
        if self.b[self.i..].starts_with(w) {
            self.i += w.len();
            Ok(())
        } else {
            Err(())
        }
    }
    fn value(&mut self, depth: usize) -> Result<Node, ()> {
        if depth > DEPTH_LIMIT {
            return Err(());
        }
        self.ws();
        let start = self.i;
        let value = match self.b.get(self.i).ok_or(())? {
            b'n' => self.word(b"null").map(|_| Json::Null)?,
            b't' => self.word(b"true").map(|_| Json::Bool(true))?,
            b'f' => self.word(b"false").map(|_| Json::Bool(false))?,
            b'"' => Json::Str(self.string()?),
            b'[' => {
                self.i += 1;
                let mut items = Vec::new();
                self.ws();
                if self.eat(b']').is_err() {
                    loop {
                        items.push(self.value(depth + 1)?);
                        self.ws();
                        if self.eat(b',').is_ok() {
                            continue;
                        }
                        self.eat(b']')?;
                        break;
                    }
                }
                Json::Arr(items)
            }
            b'{' => {
                self.i += 1;
                let mut members: Vec<(String, Node)> = Vec::new();
                self.ws();
                if self.eat(b'}').is_err() {
                    loop {
                        self.ws();
                        let key = self.string()?;
                        if members.iter().any(|(k, _)| *k == key) {
                            return Err(());
                        }
                        self.ws();
                        self.eat(b':')?;
                        let v = self.value(depth + 1)?;
                        members.push((key, v));
                        self.ws();
                        if self.eat(b',').is_ok() {
                            continue;
                        }
                        self.eat(b'}')?;
                        break;
                    }
                }
                Json::Obj(members)
            }
            b'-' | b'0'..=b'9' => Json::Num(self.number()?),
            _ => return Err(()),
        };
        Ok(Node {
            value,
            span: start..self.i,
        })
    }
    fn digits(&mut self) -> usize {
        let s = self.i;
        while self.i < self.b.len() && self.b[self.i].is_ascii_digit() {
            self.i += 1;
        }
        self.i - s
    }
    fn number(&mut self) -> Result<String, ()> {
        let start = self.i;
        let _ = self.eat(b'-');
        match self.b.get(self.i) {
            Some(b'0') => self.i += 1,
            Some(b'1'..=b'9') => {
                self.digits();
            }
            _ => return Err(()),
        }
        if self.eat(b'.').is_ok() && self.digits() == 0 {
            return Err(());
        }
        if matches!(self.b.get(self.i), Some(b'e' | b'E')) {
            self.i += 1;
            if matches!(self.b.get(self.i), Some(b'+' | b'-')) {
                self.i += 1;
            }
            if self.digits() == 0 {
                return Err(());
            }
        }
        // Only ASCII digits, signs, '.' and exponent letters were consumed.
        Ok(String::from_utf8_lossy(&self.b[start..self.i]).into_owned())
    }
    fn hex4(&mut self) -> Result<u32, ()> {
        let h = self.b.get(self.i..self.i + 4).ok_or(())?;
        if !h.iter().all(u8::is_ascii_hexdigit) {
            return Err(());
        }
        self.i += 4;
        Ok(h.iter().fold(0, |acc, c| {
            acc * 16 + (*c as char).to_digit(16).expect("a hex digit")
        }))
    }
    fn string(&mut self) -> Result<String, ()> {
        self.eat(b'"')?;
        let mut out: Vec<u8> = Vec::new();
        loop {
            let c = *self.b.get(self.i).ok_or(())?;
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e = *self.b.get(self.i).ok_or(())?;
                    self.i += 1;
                    let ch = match e {
                        b'"' => '"',
                        b'\\' => '\\',
                        b'/' => '/',
                        b'b' => '\u{8}',
                        b'f' => '\u{c}',
                        b'n' => '\n',
                        b'r' => '\r',
                        b't' => '\t',
                        b'u' => {
                            let hi = self.hex4()?;
                            let code = if (0xD800..0xDC00).contains(&hi) {
                                self.word(b"\\u")?;
                                let lo = self.hex4()?;
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return Err(());
                                }
                                0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00)
                            } else {
                                hi
                            };
                            char::from_u32(code).ok_or(())?
                        }
                        _ => return Err(()),
                    };
                    let mut buf = [0u8; 4];
                    out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                }
                0..=0x1f => return Err(()),
                _ => out.push(c),
            }
        }
        // The whole input was checked as UTF-8 and escapes decode to chars.
        String::from_utf8(out).map_err(|_| ())
    }
}

/// Parse one whole JSON document strictly.
pub fn parse(bytes: &[u8]) -> Result<Node, ()> {
    std::str::from_utf8(bytes).map_err(|_| ())?;
    let mut r = Reader { b: bytes, i: 0 };
    let node = r.value(0)?;
    r.ws();
    if r.i != bytes.len() {
        return Err(());
    }
    Ok(node)
}

fn quote(s: &str) -> String {
    serde_json::to_string(s).expect("a string serializes")
}

/// Serialize a value canonically: no whitespace, member order kept, number
/// spellings kept, strings escaped as serde_json escapes them.
fn encode(node: &Json, out: &mut String) {
    match node {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Num(n) => out.push_str(n),
        Json::Str(s) => out.push_str(&quote(s)),
        Json::Arr(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                encode(&item.value, out);
            }
            out.push(']');
        }
        Json::Obj(members) => {
            out.push('{');
            for (i, (k, v)) in members.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&quote(k));
                out.push(':');
                encode(&v.value, out);
            }
            out.push('}');
        }
    }
}

// ---- The sealed registry and redaction ----------------------------------

/// The literals a trial's values are redacted against: the fixture admin
/// bearer and every registered sentinel. Sealed once requests have drained;
/// an overflowing or unreadable registry is never used.
#[derive(Clone)]
pub struct Registry {
    literals: Vec<String>,
    bytes: usize,
    complete: bool,
}

impl std::fmt::Debug for Registry {
    // Never prints a literal.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Registry")
            .field("literals", &self.literals.len())
            .field("complete", &self.complete)
            .finish()
    }
}

impl Default for Registry {
    fn default() -> Self {
        Self::new()
    }
}

impl Registry {
    pub fn new() -> Self {
        Self {
            literals: Vec::new(),
            bytes: 0,
            complete: true,
        }
    }
    /// Register one literal. Empty literals are ignored; a duplicate counts
    /// once; past either cap the registry is marked incomplete for good.
    pub fn register(&mut self, literal: &str) {
        if literal.is_empty() || self.literals.iter().any(|l| l == literal) {
            return;
        }
        if self.literals.len() >= REGISTRY_LITERALS || literal.len() > REGISTRY_BYTES - self.bytes
        {
            self.complete = false;
            return;
        }
        self.bytes += literal.len();
        self.literals.push(literal.to_string());
    }
    /// Mark the registry unusable (a source could not be read whole).
    pub fn poison(&mut self) {
        self.complete = false;
    }
    pub fn is_complete(&self) -> bool {
        self.complete
    }
    pub fn len(&self) -> usize {
        self.literals.len()
    }
    pub fn is_empty(&self) -> bool {
        self.literals.is_empty()
    }
    /// Whether any registered literal occurs in `bytes`.
    pub fn found_in(&self, bytes: &[u8]) -> bool {
        self.literals
            .iter()
            .any(|l| bytes.windows(l.len()).any(|w| w == l.as_bytes()))
    }
    /// Replace every maximal run of bytes covered by any (possibly
    /// overlapping) occurrence of a literal with one [`MASK`].
    fn mask(&self, s: &str) -> String {
        let b = s.as_bytes();
        let mut covered = vec![false; b.len()];
        for l in &self.literals {
            let l = l.as_bytes();
            for start in 0..(b.len() + 1).saturating_sub(l.len()) {
                if &b[start..start + l.len()] == l {
                    covered[start..start + l.len()]
                        .iter_mut()
                        .for_each(|c| *c = true);
                }
            }
        }
        let mut out = String::new();
        let mut i = 0;
        while i < b.len() {
            if covered[i] {
                while i < b.len() && covered[i] {
                    i += 1;
                }
                out.push_str(MASK);
            } else {
                // Literals are UTF-8, so a covered run starts and ends on a
                // character boundary; copy the uncovered character whole.
                let ch = s[i..].chars().next().expect("a boundary");
                out.push(ch);
                i += ch.len_utf8();
            }
        }
        out
    }

    /// One string's redacted form: a string that is itself a JSON container
    /// is redacted structurally and re-encoded; a string whose percent
    /// decoding reveals a literal is redacted in its decoded form; otherwise
    /// the literal text is masked.
    fn redact_str(&self, s: &str, depth: usize) -> Result<String, Refusal> {
        let trimmed = s.trim_start();
        if (trimmed.starts_with('{') || trimmed.starts_with('['))
            && let Ok(nested) = parse(s.as_bytes())
            && self.residual(&nested.value, depth + 1)
        {
            if depth >= DEPTH_LIMIT {
                return Err(Refusal::InvalidArgumentEncoding);
            }
            let redacted = self.redact_node(&nested.value, depth + 1)?;
            let mut out = String::new();
            encode(&redacted, &mut out);
            return Ok(out);
        }
        if let Some(decoded) = percent_decoded(s)
            && self.found_in(decoded.as_bytes())
        {
            return Ok(self.mask(&decoded));
        }
        Ok(self.mask(s))
    }

    fn redact_node(&self, v: &Json, depth: usize) -> Result<Json, Refusal> {
        let node = |value| Node { value, span: 0..0 };
        Ok(match v {
            // A number cannot hold a mask: a literal in its spelling is a
            // residual the final check refuses.
            Json::Null | Json::Bool(_) | Json::Num(_) => v.clone(),
            Json::Str(s) => Json::Str(self.redact_str(s, depth)?),
            Json::Arr(items) => Json::Arr(
                items
                    .iter()
                    .map(|i| self.redact_node(&i.value, depth).map(node))
                    .collect::<Result<_, _>>()?,
            ),
            Json::Obj(members) => {
                let mut out: Vec<(String, Node)> = Vec::new();
                for (k, val) in members {
                    let key = self.redact_str(k, depth)?;
                    // Two names that collide once masked are ambiguous.
                    if out.iter().any(|(seen, _)| *seen == key) {
                        return Err(Refusal::InvalidArgumentEncoding);
                    }
                    out.push((key, node(self.redact_node(&val.value, depth)?)));
                }
                Json::Obj(out)
            }
        })
    }

    /// Whether any literal survives anywhere in a value: every decoded name
    /// and string, their percent-decoded and nested-JSON forms, and number
    /// spellings.
    fn residual(&self, v: &Json, depth: usize) -> bool {
        let text = |s: &str| {
            self.found_in(s.as_bytes())
                || percent_decoded(s).is_some_and(|d| self.found_in(d.as_bytes()))
                || (depth < DEPTH_LIMIT
                    && parse(s.as_bytes()).is_ok_and(|n| self.residual(&n.value, depth + 1)))
        };
        match v {
            Json::Null | Json::Bool(_) => false,
            Json::Num(n) => self.found_in(n.as_bytes()),
            Json::Str(s) => text(s),
            Json::Arr(items) => items.iter().any(|i| self.residual(&i.value, depth)),
            Json::Obj(members) => members
                .iter()
                .any(|(k, val)| text(k) || self.residual(&val.value, depth)),
        }
    }

    /// Redact one argument's original bytes into its checked, encoded form.
    pub fn redact(&self, original: &[u8]) -> Result<String, Refusal> {
        if original.len() > VALUE_LIMIT {
            return Err(Refusal::ValueTooLarge);
        }
        let node = parse(original).map_err(|_| Refusal::InvalidArgumentEncoding)?;
        let redacted = self.redact_node(&node.value, 0)?;
        let mut out = String::new();
        encode(&redacted, &mut out);
        if out.len() > VALUE_LIMIT {
            return Err(Refusal::ValueTooLarge);
        }
        // Validate the re-encoding: it parses back to a tree that encodes to
        // the same bytes, and no literal survives in its bytes or in any
        // decoded form (the mask itself included).
        let back = parse(out.as_bytes()).map_err(|_| Refusal::InvalidArgumentEncoding)?;
        let mut again = String::new();
        encode(&back.value, &mut again);
        if again != out {
            return Err(Refusal::InvalidArgumentEncoding);
        }
        if self.found_in(out.as_bytes()) || self.residual(&back.value, 0) {
            return Err(Refusal::ResidualSecret);
        }
        Ok(out)
    }
}

fn percent_decoded(s: &str) -> Option<String> {
    if !s.contains('%') {
        return None;
    }
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let h = b.get(i + 1..i + 3)?;
            if !h.iter().all(u8::is_ascii_hexdigit) {
                return None;
            }
            out.push(h.iter().fold(0u8, |acc, c| {
                acc * 16 + (*c as char).to_digit(16).expect("a hex digit") as u8
            }));
            i += 3;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// Build a trial's registry: the fixture admin bearer and the sentinels in
/// the operator's file, a JSON array of strings read through the bounded
/// reader. An unreadable or malformed file poisons the registry.
pub fn registry(bearer: &str, sentinels: Option<&Path>) -> Registry {
    let mut r = Registry::new();
    r.register(bearer);
    match sentinels.map(read_sentinels) {
        None => {}
        Some(Ok(list)) => list.iter().for_each(|s| r.register(s)),
        Some(Err(())) => r.poison(),
    }
    r
}

/// Read a sentinel file: a JSON array of at most [`REGISTRY_LITERALS`]
/// nonempty strings within [`REGISTRY_BYTES`] of file.
pub fn read_sentinels(path: &Path) -> Result<Vec<String>, ()> {
    let dir = path.parent().ok_or(())?;
    let name = path.file_name().and_then(|n| n.to_str()).ok_or(())?;
    let dir = artifacts::resolve_root(dir).map_err(|_| ())?;
    let bytes = artifacts::read_file(&dir, name, REGISTRY_BYTES).map_err(|_| ())?;
    let list: Vec<String> = serde_json::from_slice(&bytes).map_err(|_| ())?;
    if list.len() > REGISTRY_LITERALS || list.iter().any(String::is_empty) {
        return Err(());
    }
    Ok(list)
}

// ---- The trial-time capture ---------------------------------------------

/// The harness-owned server a call went to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Server {
    Swarmail,
    Fixture,
}

struct Observed {
    server: Server,
    tool: String,
    original: Vec<u8>,
}

#[derive(Default)]
struct CaptureState {
    calls: Vec<Observed>,
    count: usize,
    total: usize,
    latched: Option<Refusal>,
    in_flight: usize,
}

impl CaptureState {
    fn latch(&mut self, r: Refusal) {
        self.latched = Some(self.latched.map_or(r, |l| l.min(r)));
        // Once refused, no original byte is kept: the record will be absent.
        self.calls.clear();
    }
}

/// One trial's observation owner. Every harness-owned `tools/call` the agent
/// makes is reported here in request order; a call that starts while another
/// is still in flight makes the order ambiguous, and the whole trial's
/// record is withheld as `binding_mismatch`. Observation never blocks,
/// retries or alters a request.
#[derive(Clone)]
pub struct Capture {
    catalogue: Arc<BTreeSet<String>>,
    state: Arc<Mutex<CaptureState>>,
}

/// A started call; dropping it ends its in-flight span.
pub struct Started {
    state: Arc<Mutex<CaptureState>>,
}

impl Drop for Started {
    fn drop(&mut self) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.in_flight = s.in_flight.saturating_sub(1);
    }
}

impl Capture {
    /// A capture that admits Swarmail calls to `catalogue` and the fixture's
    /// `eval_http`.
    pub fn new(catalogue: BTreeSet<String>) -> Self {
        Self {
            catalogue: Arc::new(catalogue),
            state: Arc::default(),
        }
    }

    /// Record one call's tool and the exact bytes of its `arguments`.
    pub fn observe(&self, server: Server, tool: &str, original: &[u8]) -> Started {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let owned = match server {
            Server::Swarmail => self.catalogue.contains(tool),
            Server::Fixture => tool == "eval_http",
        };
        if s.in_flight > 0 {
            s.latch(Refusal::BindingMismatch);
        }
        s.in_flight += 1;
        s.count += 1;
        s.total = s.total.saturating_add(original.len());
        if !owned {
            s.latch(Refusal::UnknownTool);
        } else if original.len() > VALUE_LIMIT {
            s.latch(Refusal::ValueTooLarge);
        } else if s.count > CALL_LIMIT || s.total > TRIAL_LIMIT {
            s.latch(Refusal::TrialTooLarge);
        } else if s.latched.is_none() {
            s.calls.push(Observed {
                server,
                tool: tool.to_string(),
                original: original.to_vec(),
            });
        }
        Started {
            state: self.state.clone(),
        }
    }

    /// Latch a refusal the observer itself detected.
    pub fn refuse(&self, r: Refusal) {
        self.state
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .latch(r);
    }

    /// Scan one JSON-RPC body sent to Swarmail's `/mcp` and observe every
    /// `tools/call` in it, in order. A body that does not parse strictly but
    /// names `tools/call` cannot be read faithfully and refuses the trial.
    pub fn observe_mcp_body(&self, body: &[u8]) -> Vec<Started> {
        let Ok(root) = parse(body) else {
            if body.windows(10).any(|w| w == b"tools/call") {
                self.refuse(Refusal::InvalidArgumentEncoding);
            }
            return Vec::new();
        };
        let messages: Vec<&Node> = match &root.value {
            Json::Arr(items) => items.iter().collect(),
            _ => vec![&root],
        };
        let mut started = Vec::new();
        for m in messages {
            if m.get("method").and_then(Node::as_str) != Some("tools/call") {
                continue;
            }
            match call_parts(m) {
                Some((name, span)) => {
                    started.push(self.observe(Server::Swarmail, name, &body[span]))
                }
                None => self.refuse(Refusal::InvalidArgumentEncoding),
            }
        }
        started
    }

    /// Seal the capture against `registry` and produce the private record.
    /// The row is checked against the same live registry: a row that holds
    /// a literal can never be copied into a transcript.
    pub fn seal(&self, trial_id: &str, registry: &Registry, row: &[u8]) -> CaptureRecord {
        let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let row_sha256 = (registry.is_complete() && !registry.found_in(row)).then(|| sha256(row));
        let record = |outcome| CaptureRecord {
            schema_version: 1,
            trial_id: trial_id.to_string(),
            row_sha256: row_sha256.clone(),
            registry_literals: registry.len() as u64,
            outcome,
        };
        if !registry.is_complete() {
            return record(CaptureOutcome::Refused {
                reason: Refusal::RegistryUnavailable,
            });
        }
        if let Some(reason) = s.latched {
            return record(CaptureOutcome::Refused { reason });
        }
        let mut calls = Vec::new();
        let mut worst: Option<Refusal> = None;
        for c in &s.calls {
            match registry.redact(&c.original) {
                Ok(value) => calls.push(CapturedCall {
                    server: c.server,
                    tool: c.tool.clone(),
                    argument_bytes: c.original.len() as u64,
                    argument_sha256: sha256(&c.original),
                    value,
                }),
                Err(r) => worst = Some(worst.map_or(r, |w| w.min(r))),
            }
        }
        match worst {
            Some(reason) => record(CaptureOutcome::Refused { reason }),
            None => record(CaptureOutcome::Complete { calls }),
        }
    }
}

/// A `tools/call` request's tool name and the byte span of its arguments.
/// A call with no string name or no `arguments` member cannot be bound.
pub fn call_parts(message: &Node) -> Option<(&str, std::ops::Range<usize>)> {
    let params = message.get("params")?;
    let name = params.get("name")?.as_str()?;
    let args = params.get("arguments")?;
    Some((name, args.span.clone()))
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CapturedCall {
    pub server: Server,
    pub tool: String,
    pub argument_bytes: u64,
    pub argument_sha256: String,
    /// The redacted, checked, canonically encoded JSON value.
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum CaptureOutcome {
    Complete { calls: Vec<CapturedCall> },
    Refused { reason: Refusal },
}

/// The private intermediate one trial leaves for the report: a complete
/// redacted capture or a closed refusal, never raw bytes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CaptureRecord {
    pub schema_version: u64,
    pub trial_id: String,
    /// The SHA-256 of the finalized row's bytes when the live registry found
    /// no literal in them; `None` when it did or could not check.
    pub row_sha256: Option<String>,
    pub registry_literals: u64,
    pub outcome: CaptureOutcome,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CaptureManifest {
    schema_version: u64,
    trial_id: String,
    capture: Binding,
}

/// The bytes of a row as both the producer and the report hash them.
pub fn row_bytes(row: &TrialResult) -> Vec<u8> {
    serde_json::to_vec(row).expect("a row serializes")
}

/// Write one trial's record under `root/<trial_id>/` (exclusive 0700
/// directory, exclusive 0600 files, no clobber), the manifest last, so a
/// leaf without a manifest is never read.
pub fn write_capture(root: &Path, record: &CaptureRecord) -> Result<(), String> {
    let bytes = artifacts::encode(record, CAPTURE_LIMIT)?;
    let leaf = artifacts::create_private_dir(root, &record.trial_id)?;
    let binding = artifacts::publish(&leaf, CAPTURE_FILE, &bytes)?;
    let manifest = CaptureManifest {
        schema_version: 1,
        trial_id: record.trial_id.clone(),
        capture: binding,
    };
    artifacts::publish(
        &leaf,
        MANIFEST_FILE,
        &artifacts::encode(&manifest, artifacts::SMALL_LIMIT)?,
    )?;
    Ok(())
}

fn valid_call(c: &CapturedCall) -> bool {
    !c.tool.is_empty()
        && c.tool.len() <= 128
        && is_hash(&c.argument_sha256)
        && c.argument_bytes <= VALUE_LIMIT as u64
        && c.value.len() <= VALUE_LIMIT
        && parse(c.value.as_bytes()).is_ok()
}

/// Read one trial's record back through the bounded, no-follow reader and
/// check its manifest binding. `None` is a capture the report cannot use:
/// missing, substituted or malformed.
pub fn read_capture(root: &Path, trial_id: &str) -> Option<CaptureRecord> {
    if !artifacts::generated_id(trial_id) {
        return None;
    }
    let leaf = root.join(trial_id);
    let manifest = artifacts::read_file(&leaf, MANIFEST_FILE, artifacts::SMALL_LIMIT).ok()?;
    let manifest: CaptureManifest = crate::provenance::decode(&manifest).ok()?;
    if manifest.schema_version != 1
        || manifest.trial_id != trial_id
        || manifest.capture.path != CAPTURE_FILE
    {
        return None;
    }
    let bytes = artifacts::read_file(&leaf, CAPTURE_FILE, CAPTURE_LIMIT).ok()?;
    if bytes.len() as u64 != manifest.capture.length || sha256(&bytes) != manifest.capture.sha256 {
        return None;
    }
    let record: CaptureRecord = crate::provenance::decode(&bytes).ok()?;
    let calls_ok = match &record.outcome {
        CaptureOutcome::Complete { calls } => {
            calls.len() <= CALL_LIMIT && calls.iter().all(valid_call)
        }
        CaptureOutcome::Refused { .. } => true,
    };
    (record.schema_version == 1
        && record.trial_id == trial_id
        && record.row_sha256.as_deref().is_none_or(is_hash)
        && calls_ok)
        .then_some(record)
}

// ---- The report-side join -----------------------------------------------

/// Which harness-owned tool a journal tool name denotes, if any. Accepted
/// spellings: `mcp__<server>__<tool>`, `<server>__<tool>`, `<server>.<tool>`,
/// `<server>/<tool>`, `<server>:<tool>`, and a bare `eval_http`.
pub fn owned_tool(name: &str) -> Option<(Server, &str)> {
    if name == "eval_http" {
        return Some((Server::Fixture, name));
    }
    let rest = name.strip_prefix("mcp__").unwrap_or(name);
    for (prefix, server) in [("swarmail", Server::Swarmail), ("fixture", Server::Fixture)] {
        let Some(after) = rest.strip_prefix(prefix) else {
            continue;
        };
        for sep in ["__", ".", "/", ":"] {
            if let Some(tool) = after.strip_prefix(sep).filter(|t| !t.is_empty()) {
                return Some((server, tool));
            }
        }
    }
    None
}

#[derive(Serialize)]
struct CompanionCall<'a> {
    ordinal: u64,
    tool: String,
    owned: bool,
    argument_bytes: u64,
    argument_sha256: &'a str,
    result_bytes: u64,
    result_sha256: &'a str,
    value: Value,
}

/// A verified companion, kept in memory for transcript selection.
#[derive(Debug, Clone)]
pub struct Companion {
    pub trial_id: String,
    pub row_sha256: Option<String>,
    /// The ordered calls as published: structural fields plus `value`.
    pub calls: Vec<Value>,
}

/// Bind a capture to the journal's structural sequence. Every journal call
/// a harness-owned name denotes must meet the next captured call with the
/// same server, tool, length and digest, and no captured call may be left
/// over. A call to any other tool is kept structural, with a `null` value.
pub fn bind(structural: &[ToolCall], record: &CaptureRecord) -> Result<Companion, Refusal> {
    let calls = match &record.outcome {
        CaptureOutcome::Refused { reason } => return Err(*reason),
        CaptureOutcome::Complete { calls } => calls,
    };
    let mut captured = calls.iter();
    let mut out = Vec::new();
    for call in structural {
        let owned = owned_tool(&call.tool);
        let value = match owned {
            None => Value::Null,
            Some((server, tool)) => {
                let c = captured.next().ok_or(Refusal::BindingMismatch)?;
                if c.server != server
                    || c.tool != tool
                    || c.argument_bytes != call.argument_bytes
                    || c.argument_sha256 != call.argument_sha256
                {
                    return Err(Refusal::BindingMismatch);
                }
                serde_json::from_str(&c.value).map_err(|_| Refusal::InvalidArgumentEncoding)?
            }
        };
        out.push(
            serde_json::to_value(CompanionCall {
                ordinal: call.ordinal,
                tool: journal::bounded_tool_name(&call.tool),
                owned: owned.is_some(),
                argument_bytes: call.argument_bytes,
                argument_sha256: &call.argument_sha256,
                result_bytes: call.result_bytes,
                result_sha256: &call.result_sha256,
                value,
            })
            .expect("a call serializes"),
        );
    }
    if captured.next().is_some() {
        return Err(Refusal::BindingMismatch);
    }
    Ok(Companion {
        trial_id: record.trial_id.clone(),
        row_sha256: record.row_sha256.clone(),
        calls: out,
    })
}

/// What the value mode decided for one trial.
#[derive(Debug)]
pub enum Disposition {
    Published(Binding, Box<Companion>),
    Absent(String),
}

impl Disposition {
    /// The trial's entry in the value index.
    pub fn index_entry(&self) -> Value {
        match self {
            Self::Published(b, _) => json!({"path": b.path, "length": b.length, "sha256": b.sha256}),
            Self::Absent(reason) => json!({"absent": reason}),
        }
    }
}

/// Decide and publish one trial's companion beside its structural record.
/// `upstream` is the journal's own absence for the trial (built-in agent,
/// no entry, an unavailable tool-call fact): it is propagated first. Then,
/// for a present fact, the checks run in [`Refusal`] order.
pub fn publish_companion(
    dir: &Path,
    capture_root: &Path,
    facts: &journal::TrialFacts,
    structural: &Value,
) -> Disposition {
    let absent = |r: Refusal| Disposition::Absent(r.as_str().into());
    let Fact::Present { value: calls } = &facts.tool_calls else {
        // The caller passes only present facts; anything else is upstream.
        return absent(Refusal::CaptureMissing);
    };
    let Some(record) = read_capture(capture_root, &facts.trial_id) else {
        return absent(Refusal::CaptureMissing);
    };
    let companion = match bind(calls, &record) {
        Ok(c) => c,
        Err(r) => return absent(r),
    };
    // A structural record that was never written, or that is cut, cannot
    // carry a companion: the binding would be to a missing or partial file.
    let leaf = dir.join(&facts.trial_id);
    if structural.get("absent").is_some() {
        return absent(Refusal::EvidenceWriteFailed);
    }
    let whole = artifacts::read_file(&leaf, "tool-calls.json", journal::EVIDENCE_LIMIT)
        .ok()
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .map(|v| v["truncated"] == json!(false));
    match whole {
        Some(true) => {}
        Some(false) => return absent(Refusal::TrialTooLarge),
        None => return absent(Refusal::EvidenceWriteFailed),
    }
    let doc = json!({
        "schema_version": 1,
        "trial_id": facts.trial_id,
        "run_id": facts.run_id,
        "structural": structural,
        "redaction": {"mask": MASK, "registry_literals": record.registry_literals},
        "model_text": "not_retained",
        "tool_calls": companion.calls,
    });
    let Ok(bytes) = artifacts::encode(&doc, COMPANION_LIMIT) else {
        return absent(Refusal::TrialTooLarge);
    };
    match artifacts::publish(&leaf, COMPANION_FILE, &bytes) {
        Ok(binding) => Disposition::Published(
            Binding {
                path: format!("{}/{}", facts.trial_id, binding.path),
                ..binding
            },
            Box::new(companion),
        ),
        Err(_) => absent(Refusal::EvidenceWriteFailed),
    }
}

// ---- Transcript selection -----------------------------------------------

fn class_of(row: &TrialResult) -> Option<FailureClass> {
    match row.outcome {
        Outcome::Pass => None,
        Outcome::Fail => row.failure_class,
        Outcome::HarnessError => Some(row.failure_class.unwrap_or(FailureClass::HarnessError)),
    }
}

/// Write the bounded `transcripts/` section. Failing rows (harness errors
/// included) are counted per class before any evidence filter; classes are
/// visited by decreasing count, ties by name; within a class trial ids are
/// taken ascending until [`PER_CLASS`] eligible trials whose leaves fit are
/// written. A leaf over its cap is skipped with a typed count and the next
/// eligible trial takes its place.
pub fn write_transcripts(
    dir: &Path,
    rows: &[TrialResult],
    companions: &BTreeMap<String, Companion>,
) -> Result<Binding, String> {
    let mut by_class: BTreeMap<&'static str, Vec<&TrialResult>> = BTreeMap::new();
    for row in rows {
        if let Some(class) = class_of(row) {
            by_class.entry(class.as_str()).or_default().push(row);
        }
    }
    let mut order: Vec<(&'static str, Vec<&TrialResult>)> = by_class.into_iter().collect();
    order.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then(a.0.cmp(b.0)));
    let tdir = artifacts::create_private_dir(dir, TRANSCRIPTS_DIR)?;
    let mut classes = Vec::new();
    for (class, mut members) in order {
        members.sort_by(|a, b| a.trial_id.as_bytes().cmp(b.trial_id.as_bytes()));
        let total = members.len();
        let mut eligible = 0u64;
        let mut skipped: BTreeMap<&'static str, u64> = BTreeMap::new();
        let mut selected = Vec::new();
        for row in members {
            let Some(c) = companions.get(&row.trial_id) else {
                continue;
            };
            let bytes = row_bytes(row);
            // Only a row the trial's live registry checked is ever copied.
            if c.row_sha256.as_deref() != Some(sha256(&bytes).as_str()) {
                *skipped.entry("row_unverified").or_default() += 1;
                continue;
            }
            eligible += 1;
            if selected.len() == PER_CLASS {
                continue;
            }
            let row_value: Value = serde_json::from_slice(&bytes).expect("a row is JSON");
            let leaf = json!({
                "schema_version": 1,
                "trial_id": row.trial_id,
                "failure_class": class,
                "outcome": row.outcome,
                "model_text": "not_retained",
                "row": row_value,
                "tool_calls": c.calls,
            });
            let Ok(leaf_bytes) = artifacts::encode(&leaf, LEAF_LIMIT) else {
                *skipped.entry("leaf_too_large").or_default() += 1;
                continue;
            };
            let binding = artifacts::publish(&tdir, &format!("{}.json", row.trial_id), &leaf_bytes)?;
            selected.push(json!({
                "trial_id": row.trial_id,
                "path": format!("{TRANSCRIPTS_DIR}/{}", binding.path),
                "length": binding.length,
                "sha256": binding.sha256,
            }));
        }
        let mut entry = Map::new();
        entry.insert("class".into(), json!(class));
        entry.insert("total".into(), json!(total));
        entry.insert("eligible".into(), json!(eligible));
        entry.insert("selected".into(), json!(selected.len()));
        entry.insert("skipped".into(), json!(skipped));
        entry.insert("transcripts".into(), Value::Array(selected));
        classes.push(Value::Object(entry));
    }
    let index = json!({
        "schema_version": 1,
        "rule": "failing rows per class; classes by decreasing count, then name; trial ids ascending; up to 3 eligible per class",
        "model_text": "not_retained",
        "classes": classes,
    });
    let bytes = artifacts::encode(&index, INDEX_LIMIT)?;
    let binding = artifacts::publish(&tdir, TRANSCRIPT_INDEX, &bytes)?;
    Ok(Binding {
        path: format!("{TRANSCRIPTS_DIR}/{}", binding.path),
        ..binding
    })
}

// ---- The rendered page --------------------------------------------------

/// Escape untrusted text for a Markdown table cell: every ASCII punctuation
/// mark is backslash-escaped and control characters become U+FFFD, so a
/// label can neither end a cell nor open markup.
pub fn escape_cell(s: &str) -> String {
    let mut out = String::new();
    for ch in s.chars() {
        if ch.is_control() {
            out.push('\u{fffd}');
        } else {
            if ch.is_ascii_punctuation() {
                out.push('\\');
            }
            out.push(ch);
        }
    }
    out
}

/// One group's tokens-per-success figure: the number, or `null` and why.
/// Never a dash, a blank or a zero standing for an unavailable figure.
/// Without a journal, a paid group with scored rows is `incomplete_join`
/// (no fact was joined) and an empty scored population is `no_passes`.
pub fn tokens_figure(group: &Value) -> (Option<f64>, &'static str) {
    if let Some(journal) = group.get("journal") {
        let reason = match journal["tokens_per_success_reason"].as_str() {
            Some("no_passes") => "no_passes",
            Some("sum_overflow") => "sum_overflow",
            _ => "incomplete_join",
        };
        return match journal["tokens_per_success"].as_f64() {
            Some(v) => (Some(v), ""),
            None => (None, reason),
        };
    }
    let builtin = group["tool_surfaces"]
        .as_array()
        .is_some_and(|s| s.iter().all(|x| x["kind"] == "builtin_not_applicable"));
    if builtin {
        (None, "builtin_agent")
    } else if group["scored"].as_u64() == Some(0) {
        (None, "no_passes")
    } else {
        (None, "incomplete_join")
    }
}

/// The private Markdown page rendered from the report's group objects.
pub fn render_page(report: &Value) -> String {
    let mut out = String::from(
        "# Eval report\n\nTokens per success is the scored attempts' input plus output tokens over their passes (decision 0119 §3); cached tokens are reported beside it, never added. A figure that cannot be computed is `null` with its reason, never a dash or a zero.\n\n| task | agent | attempts | passes | fails | harness errors | tokens per success | cached tokens |\n|---|---|---|---|---|---|---|---|\n",
    );
    for g in report["groups"].as_array().into_iter().flatten() {
        let text = |v: &Value| escape_cell(v.as_str().unwrap_or("(none)"));
        let figure = match tokens_figure(g) {
            (Some(v), _) => format!("{v}"),
            (None, reason) => format!("null ({})", escape_cell(reason)),
        };
        let cached = match g["journal"]["tokens"]["cached"]["sum"].as_u64() {
            Some(n) => n.to_string(),
            None => "null".into(),
        };
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} |\n",
            text(&g["task"]),
            text(&g["agent"]),
            g["attempts"],
            g["passes"],
            g["fails"],
            g["harness_errors"],
            figure,
            cached,
        ));
    }
    out
}

#[cfg(test)]
mod tests;
