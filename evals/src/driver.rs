//! The caller side of brokkr's forge-driver/v1 stdio protocol (decision 0119's
//! 2026-11-23 addendum, route (b); proposed 0138; specs/nested-driver-usage).
//!
//! `swarmail-evals` acts as the nested driver's engine: it sends `hello`, then
//! `start{effect_id, attempt_id, seat, input}` once `capabilities` arrives, then
//! `shutdown`; the driver answers `capabilities`, `accepted`, any number of
//! `checkpoint`, then one `result`. The shape is brokkr v0.15.2's
//! `docs/guides/driver-authoring.md`: every frame carries `proto`, a unique
//! `msg_id` and `type`; `accepted`, `checkpoint` and `result` carry the start's
//! `effect_id` and `attempt_id`; a checkpoint's facts are its `data` object; a
//! failed result before `accepted` is a refusal to start. The reader is
//! bounded and strict, and every error is a fixed code, never peer bytes.
//! Usage is read from the checkpoints by `crate::usage`, never here.

use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};
use std::fmt;
use std::io::BufRead;

/// The protocol name every frame carries in `proto`.
pub const PROTO: &str = "forge-driver/v1";
/// What `hello` names as the engine: this harness and its version.
pub const ENGINE_VERSION: &str = concat!("swarmail-evals ", env!("CARGO_PKG_VERSION"));
/// One frame's cap, newline excluded, enforced before the bytes are kept.
pub const MAX_FRAME_BYTES: usize = 64 * 1024;
/// The cap over every frame of one exchange.
pub const MAX_TOTAL_BYTES: usize = 4 * 1024 * 1024;
/// The cap on frames in one exchange.
pub const MAX_FRAMES: usize = 4096;
/// The deepest JSON nesting a frame may carry (the envelope is depth 1).
pub const MAX_DEPTH: usize = 16;

/// Why an exchange failed, as a fixed code: never a peer's bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProtocolError {
    Io,
    FrameTooLarge,
    TotalTooLarge,
    TooManyFrames,
    Truncated,
    NotUtf8,
    NotJson,
    NotObject,
    DuplicateKey,
    TooDeep,
    WrongProto,
    MissingField,
    UnknownType,
    OutOfOrder,
    IdentityMismatch,
    AfterResult,
    DuplicateMsgId,
    BadStatus,
}

impl ProtocolError {
    pub fn code(self) -> &'static str {
        match self {
            Self::Io => "driver_io",
            Self::FrameTooLarge => "driver_frame_too_large",
            Self::TotalTooLarge => "driver_output_too_large",
            Self::TooManyFrames => "driver_too_many_frames",
            Self::Truncated => "driver_frame_truncated",
            Self::NotUtf8 => "driver_frame_not_utf8",
            Self::NotJson => "driver_frame_not_json",
            Self::NotObject => "driver_frame_not_object",
            Self::DuplicateKey => "driver_frame_duplicate_key",
            Self::TooDeep => "driver_frame_too_deep",
            Self::WrongProto => "driver_proto_mismatch",
            Self::MissingField => "driver_field_missing",
            Self::UnknownType => "driver_type_unknown",
            Self::OutOfOrder => "driver_out_of_order",
            Self::IdentityMismatch => "driver_identity_mismatch",
            Self::AfterResult => "driver_frame_after_result",
            Self::DuplicateMsgId => "driver_msg_id_duplicate",
            Self::BadStatus => "driver_result_status_invalid",
        }
    }
}

/// Every code a nested receipt's disposition may carry, as a static string:
/// `completed`, a disposition's harness error or a protocol error's code.
pub fn static_code(code: &str) -> Option<&'static str> {
    use ProtocolError::*;
    let dispositions = [
        Disposition::Completed,
        Disposition::NotLaunched { refused: false },
        Disposition::NotLaunched { refused: true },
        Disposition::ResultMissing,
        Disposition::Failed,
        Disposition::InvalidResult,
    ]
    .map(Disposition::code);
    let protocol = [
        Io,
        FrameTooLarge,
        TotalTooLarge,
        TooManyFrames,
        Truncated,
        NotUtf8,
        NotJson,
        NotObject,
        DuplicateKey,
        TooDeep,
        WrongProto,
        MissingField,
        UnknownType,
        OutOfOrder,
        IdentityMismatch,
        AfterResult,
        DuplicateMsgId,
        BadStatus,
    ]
    .map(ProtocolError::code);
    dispositions
        .into_iter()
        .chain(protocol)
        .find(|c| *c == code)
}

/// The eight `start.input` keys the smoke found sufficient for every family.
#[derive(Debug, Clone, PartialEq)]
pub struct StartInput {
    pub feature: String,
    pub phase: String,
    pub seat: String,
    pub role_path: String,
    pub workdir: String,
    pub result_path: String,
    pub allowed_results: Vec<String>,
    pub context: Value,
}

/// The engine identities one start binds every later frame to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub effect_id: String,
    pub attempt_id: String,
    pub seat: String,
}

fn envelope(kind: &str, msg_id: &str) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("proto".into(), PROTO.into());
    m.insert("msg_id".into(), msg_id.into());
    m.insert("type".into(), kind.into());
    m
}

fn line(m: Map<String, Value>) -> Vec<u8> {
    let mut out = serde_json::to_vec(&Value::Object(m)).expect("a JSON map always encodes");
    out.push(b'\n');
    out
}

/// `hello`, one line.
pub fn hello(msg_id: &str) -> Vec<u8> {
    let mut m = envelope("hello", msg_id);
    m.insert("engine_version".into(), ENGINE_VERSION.into());
    line(m)
}

/// `shutdown`, one line.
pub fn shutdown(msg_id: &str) -> Vec<u8> {
    line(envelope("shutdown", msg_id))
}

/// `start{effect_id, attempt_id, seat, input}`, one line, with exactly the
/// eight input keys.
pub fn start(msg_id: &str, id: &Identity, input: &StartInput) -> Vec<u8> {
    let mut m = envelope("start", msg_id);
    m.insert("effect_id".into(), id.effect_id.as_str().into());
    m.insert("attempt_id".into(), id.attempt_id.as_str().into());
    m.insert("seat".into(), id.seat.as_str().into());
    let mut i = Map::new();
    i.insert("feature".into(), input.feature.as_str().into());
    i.insert("phase".into(), input.phase.as_str().into());
    i.insert("seat".into(), input.seat.as_str().into());
    i.insert("role_path".into(), input.role_path.as_str().into());
    i.insert("workdir".into(), input.workdir.as_str().into());
    i.insert("result_path".into(), input.result_path.as_str().into());
    i.insert(
        "allowed_results".into(),
        input
            .allowed_results
            .iter()
            .map(|s| Value::from(s.as_str()))
            .collect(),
    );
    i.insert("context".into(), input.context.clone());
    m.insert("input".into(), Value::Object(i));
    line(m)
}

/// A JSON value decoded with duplicate keys refused at every depth and the
/// nesting bounded, so a frame means one thing to every reader.
struct Strict {
    depth: usize,
}

const DUPLICATE: &str = "\u{0}duplicate";
const DEEP: &str = "\u{0}deep";

impl<'de> DeserializeSeed<'de> for Strict {
    type Value = Value;
    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> Result<Value, D::Error> {
        d.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for Strict {
    type Value = Value;
    fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str("JSON")
    }
    fn visit_bool<E>(self, v: bool) -> Result<Value, E> {
        Ok(v.into())
    }
    fn visit_i64<E>(self, v: i64) -> Result<Value, E> {
        Ok(v.into())
    }
    fn visit_u64<E>(self, v: u64) -> Result<Value, E> {
        Ok(v.into())
    }
    fn visit_f64<E>(self, v: f64) -> Result<Value, E> {
        Ok(serde_json::Number::from_f64(v).map_or(Value::Null, Value::Number))
    }
    fn visit_str<E>(self, v: &str) -> Result<Value, E> {
        Ok(v.into())
    }
    fn visit_string<E>(self, v: String) -> Result<Value, E> {
        Ok(v.into())
    }
    fn visit_unit<E>(self) -> Result<Value, E> {
        Ok(Value::Null)
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
        let depth = self.depth + 1;
        if depth > MAX_DEPTH {
            return Err(A::Error::custom(DEEP));
        }
        let mut out = Vec::new();
        while let Some(v) = seq.next_element_seed(Strict { depth })? {
            out.push(v);
        }
        Ok(Value::Array(out))
    }
    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
        let depth = self.depth + 1;
        if depth > MAX_DEPTH {
            return Err(A::Error::custom(DEEP));
        }
        let mut out = Map::new();
        while let Some(key) = map.next_key::<String>()? {
            let value = map.next_value_seed(Strict { depth })?;
            if out.insert(key, value).is_some() {
                return Err(A::Error::custom(DUPLICATE));
            }
        }
        Ok(Value::Object(out))
    }
}

/// One frame decoded strictly: UTF-8, one JSON object, no duplicate key,
/// bounded depth, the protocol name, a string `msg_id` and `type`.
pub fn decode(bytes: &[u8]) -> Result<Map<String, Value>, ProtocolError> {
    let text = std::str::from_utf8(bytes).map_err(|_| ProtocolError::NotUtf8)?;
    let mut de = serde_json::Deserializer::from_str(text);
    let value = Strict { depth: 0 }.deserialize(&mut de).map_err(|e| {
        let msg = e.to_string();
        if msg.starts_with(DUPLICATE) {
            ProtocolError::DuplicateKey
        } else if msg.starts_with(DEEP) {
            ProtocolError::TooDeep
        } else {
            ProtocolError::NotJson
        }
    })?;
    de.end().map_err(|_| ProtocolError::NotJson)?;
    let Value::Object(m) = value else {
        return Err(ProtocolError::NotObject);
    };
    match m.get("proto") {
        Some(Value::String(p)) if p == PROTO => {}
        Some(_) => return Err(ProtocolError::WrongProto),
        None => return Err(ProtocolError::MissingField),
    }
    for key in ["msg_id", "type"] {
        if !matches!(m.get(key), Some(Value::String(_))) {
            return Err(ProtocolError::MissingField);
        }
    }
    Ok(m)
}

/// A bounded reader of newline-delimited frames: each frame's cap holds
/// before its bytes are kept, and the exchange's byte and frame totals are
/// capped too. A final line without its newline is truncation.
pub struct FrameReader<R> {
    inner: R,
    total: usize,
    frames: usize,
}

impl<R: BufRead> FrameReader<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            total: 0,
            frames: 0,
        }
    }

    /// The next frame, `Ok(None)` at a clean end of stream.
    pub fn next_frame(&mut self) -> Result<Option<Map<String, Value>>, ProtocolError> {
        let mut buf = Vec::new();
        loop {
            let chunk = self.inner.fill_buf().map_err(|_| ProtocolError::Io)?;
            if chunk.is_empty() {
                return if buf.is_empty() {
                    Ok(None)
                } else {
                    Err(ProtocolError::Truncated)
                };
            }
            let (take, done) = match chunk.iter().position(|b| *b == b'\n') {
                Some(n) => (n, true),
                None => (chunk.len(), false),
            };
            if buf.len() + take > MAX_FRAME_BYTES {
                return Err(ProtocolError::FrameTooLarge);
            }
            let used = take + usize::from(done);
            if self.total + used > MAX_TOTAL_BYTES {
                return Err(ProtocolError::TotalTooLarge);
            }
            buf.extend_from_slice(&chunk[..take]);
            self.inner.consume(used);
            self.total += used;
            if done {
                break;
            }
        }
        self.frames += 1;
        if self.frames > MAX_FRAMES {
            return Err(ProtocolError::TooManyFrames);
        }
        decode(&buf).map(Some)
    }
}

/// Where an exchange stands; each driver frame may only move it forward.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    AwaitCapabilities,
    AwaitAccepted,
    Running,
    Finished,
}

/// What the driver did, as far as its frames prove (v0.15.2's guide,
/// "`accepted` is the load-bearing message").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    /// No `accepted`: nothing was launched. `refused` when the driver said so
    /// with a failed result. A driver that exits 0 here is a harness error,
    /// never the agent giving up.
    NotLaunched { refused: bool },
    /// Accepted, but the stream ended before its `result`: what the session
    /// did is unknown.
    ResultMissing,
    /// Accepted, then a `failed` result: the session may have spent.
    Failed,
    /// Accepted, then a `succeeded` result whose payload is not an object
    /// naming one of the allowed results.
    InvalidResult,
    /// Accepted, then a `succeeded` result naming an allowed result.
    Completed,
}

impl Disposition {
    /// True once the driver proved a session began.
    pub fn launched(self) -> bool {
        !matches!(self, Self::NotLaunched { .. })
    }
    /// The trial's harness-error reason, `None` only for a completed session.
    pub fn harness_error(self) -> Option<&'static str> {
        match self {
            Self::NotLaunched { refused: false } => Some("driver_launched_nothing"),
            Self::NotLaunched { refused: true } => Some("driver_refused_start"),
            Self::ResultMissing => Some("driver_result_missing"),
            Self::Failed => Some("driver_session_failed"),
            Self::InvalidResult => Some("driver_result_invalid"),
            Self::Completed => None,
        }
    }
    /// The receipt word: `completed` or the harness-error code.
    pub fn code(self) -> &'static str {
        self.harness_error().unwrap_or("completed")
    }
}

fn string_field(m: &Map<String, Value>, key: &str) -> Result<String, ProtocolError> {
    match m.get(key) {
        Some(Value::String(s)) => Ok(s.clone()),
        _ => Err(ProtocolError::MissingField),
    }
}

/// The driver's `capabilities`, as v0.15.2 requires them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Capabilities {
    pub driver: String,
    pub version: String,
}

/// The ordered exchange's checker: feed it every driver frame; it says when
/// `start` and `shutdown` are due and what the driver did at end of stream.
pub struct Exchange {
    id: Identity,
    allowed: Vec<String>,
    stage: Stage,
    msg_ids: std::collections::BTreeSet<String>,
    capabilities: Option<Capabilities>,
    checkpoints: Vec<Map<String, Value>>,
    result: Option<Disposition>,
    refused: bool,
}

impl Exchange {
    pub fn new(id: Identity, allowed: Vec<String>) -> Self {
        Self {
            id,
            allowed,
            stage: Stage::AwaitCapabilities,
            msg_ids: Default::default(),
            capabilities: None,
            checkpoints: Vec::new(),
            result: None,
            refused: false,
        }
    }

    /// True once `capabilities` arrived and `start` may be sent.
    pub fn ready_to_start(&self) -> bool {
        self.stage == Stage::AwaitAccepted
    }

    /// True once the driver accepted the attempt (or ran past it).
    pub fn accepted(&self) -> bool {
        matches!(self.stage, Stage::Running) || self.result.is_some()
    }

    /// True once the `result` arrived and `shutdown` may be sent.
    pub fn finished(&self) -> bool {
        self.stage == Stage::Finished
    }

    /// The driver's declared name and version.
    pub fn capabilities(&self) -> Option<&Capabilities> {
        self.capabilities.as_ref()
    }

    /// Every checkpoint's `data` object, in order; the envelope is dropped.
    pub fn checkpoints(&self) -> &[Map<String, Value>] {
        &self.checkpoints
    }

    fn check_identity(&self, m: &Map<String, Value>) -> Result<(), ProtocolError> {
        for (key, want) in [
            ("effect_id", &self.id.effect_id),
            ("attempt_id", &self.id.attempt_id),
        ] {
            match m.get(key) {
                Some(Value::String(got)) if got == want => {}
                None => return Err(ProtocolError::MissingField),
                Some(_) => return Err(ProtocolError::IdentityMismatch),
            }
        }
        Ok(())
    }

    fn terminal(&self, m: &Map<String, Value>) -> Result<Disposition, ProtocolError> {
        match m.get("status").and_then(Value::as_str) {
            Some("failed") => Ok(Disposition::Failed),
            Some("succeeded") => Ok(match m.get("result") {
                Some(Value::Object(r))
                    if r.get("result")
                        .and_then(Value::as_str)
                        .is_some_and(|s| self.allowed.iter().any(|a| a == s)) =>
                {
                    Disposition::Completed
                }
                _ => Disposition::InvalidResult,
            }),
            _ => Err(ProtocolError::BadStatus),
        }
    }

    /// One decoded driver frame.
    pub fn receive(&mut self, mut m: Map<String, Value>) -> Result<(), ProtocolError> {
        if self.stage == Stage::Finished {
            return Err(ProtocolError::AfterResult);
        }
        let msg_id = m
            .get("msg_id")
            .and_then(Value::as_str)
            .ok_or(ProtocolError::MissingField)?;
        if !self.msg_ids.insert(msg_id.to_string()) {
            return Err(ProtocolError::DuplicateMsgId);
        }
        let kind = m.get("type").and_then(Value::as_str).unwrap_or_default();
        match (kind, self.stage) {
            ("capabilities", Stage::AwaitCapabilities) => {
                let caps = Capabilities {
                    driver: string_field(&m, "driver")?,
                    version: string_field(&m, "version")?,
                };
                match m.get("supports") {
                    Some(Value::Array(a)) if a.iter().all(Value::is_string) => {}
                    _ => return Err(ProtocolError::MissingField),
                }
                self.capabilities = Some(caps);
                self.stage = Stage::AwaitAccepted;
            }
            ("accepted", Stage::AwaitAccepted) => {
                self.check_identity(&m)?;
                self.stage = Stage::Running;
            }
            ("checkpoint", Stage::Running) => {
                self.check_identity(&m)?;
                match m.remove("data") {
                    Some(Value::Object(data)) => self.checkpoints.push(data),
                    _ => return Err(ProtocolError::MissingField),
                }
            }
            ("result", Stage::AwaitAccepted) => {
                // A pre-session refusal: a failed result with no accepted
                // and no checkpoint. Anything else here is out of order.
                self.check_identity(&m)?;
                if self.terminal(&m)? != Disposition::Failed {
                    return Err(ProtocolError::OutOfOrder);
                }
                self.refused = true;
                self.stage = Stage::Finished;
            }
            ("result", Stage::Running) => {
                self.check_identity(&m)?;
                self.result = Some(self.terminal(&m)?);
                self.stage = Stage::Finished;
            }
            ("capabilities" | "accepted" | "checkpoint" | "result", _) => {
                return Err(ProtocolError::OutOfOrder);
            }
            _ => return Err(ProtocolError::UnknownType),
        }
        Ok(())
    }

    /// The disposition at end of stream.
    pub fn finish(&self) -> Disposition {
        match (self.result, self.stage) {
            (Some(d), _) => d,
            (None, Stage::Running) => Disposition::ResultMissing,
            (None, _) => Disposition::NotLaunched {
                refused: self.refused,
            },
        }
    }
}

/// Drive a whole exchange over one reader: `hello` first, `start` once the
/// capabilities arrive, `shutdown` after the result or at end of stream,
/// every frame decoded and checked in order. `send` gets each frame and
/// whether it is the last: after `shutdown` the caller closes the driver's
/// stdin, as the engine does. `on_accepted` runs once, when the driver
/// proves a session began. Deadlines and the process belong to
/// the caller; on a protocol error the exchange so far is returned with it.
pub fn run<R: BufRead>(
    reader: R,
    id: Identity,
    input: &StartInput,
    mut send: impl FnMut(Vec<u8>, bool) -> std::io::Result<()>,
    mut on_accepted: impl FnMut(),
) -> (Exchange, Result<Disposition, ProtocolError>) {
    let mut seq = 0u32;
    let mut next_id = || {
        seq += 1;
        format!("swarmail-evals-{seq}")
    };
    let mut ex = Exchange::new(id, input.allowed_results.clone());
    let (mut started, mut shut, mut told) = (false, false, false);
    let body = || -> Result<(), ProtocolError> {
        send(hello(&next_id()), false).map_err(|_| ProtocolError::Io)?;
        let mut frames = FrameReader::new(reader);
        while let Some(frame) = frames.next_frame()? {
            ex.receive(frame)?;
            if ex.accepted() && !told {
                told = true;
                on_accepted();
            }
            if ex.ready_to_start() && !started {
                started = true;
                send(start(&next_id(), &ex.id, input), false).map_err(|_| ProtocolError::Io)?;
            }
            if ex.finished() && !shut {
                shut = true;
                send(shutdown(&next_id()), true).map_err(|_| ProtocolError::Io)?;
            }
        }
        if !shut {
            // Shutdown is sent once, after the result or after EOF; a
            // driver that is already gone cannot read it.
            let _ = send(shutdown(&next_id()), true);
        }
        Ok(())
    };
    let outcome = body();
    let disposition = outcome.map(|()| ex.finish());
    (ex, disposition)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Cursor;

    fn id() -> Identity {
        Identity {
            effect_id: "e1".into(),
            attempt_id: "a1".into(),
            seat: "trial".into(),
        }
    }

    fn input() -> StartInput {
        StartInput {
            feature: "f".into(),
            phase: "trial".into(),
            seat: "trial".into(),
            role_path: "/w/charter.md".into(),
            workdir: "/w".into(),
            result_path: "/w/result.json".into(),
            allowed_results: vec!["complete".into(), "broken".into()],
            context: json!({"goal": "g"}),
        }
    }

    fn frame(kind: &str, extra: Value) -> String {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut v = json!({"proto": PROTO, "msg_id": format!("m{n}"), "type": kind});
        for (k, x) in extra.as_object().unwrap() {
            v[k] = x.clone();
        }
        format!("{v}\n")
    }

    fn ids() -> Value {
        json!({"effect_id": "e1", "attempt_id": "a1"})
    }
    fn with_ids(extra: Value) -> Value {
        let mut v = ids();
        for (k, x) in extra.as_object().unwrap() {
            v[k] = x.clone();
        }
        v
    }
    fn caps() -> String {
        frame(
            "capabilities",
            json!({"driver": "claude-code", "version": "0.15.2", "supports": ["resume"], "probe": true}),
        )
    }
    fn checkpoint(data: Value) -> String {
        frame("checkpoint", with_ids(json!({ "data": data })))
    }
    fn result(status: &str, payload: Value) -> String {
        frame(
            "result",
            with_ids(json!({"status": status, "result": payload})),
        )
    }

    type Outcome = Result<Disposition, ProtocolError>;

    fn exchange(stream: &str) -> (Exchange, Outcome, Vec<Value>, usize) {
        let mut sent = Vec::new();
        let mut accepted = 0;
        let (ex, out) = run(
            Cursor::new(stream.as_bytes().to_vec()),
            id(),
            &input(),
            |b, last| {
                assert_eq!(b.last(), Some(&b'\n'));
                let frame: Value = serde_json::from_slice(&b).unwrap();
                assert_eq!(last, frame["type"] == "shutdown");
                sent.push(frame);
                Ok(())
            },
            || accepted += 1,
        );
        (ex, out, sent, accepted)
    }

    #[test]
    fn the_whole_exchange_runs_in_order_and_sends_exactly_the_eight_keys() {
        let stream = [
            caps(),
            frame("accepted", with_ids(json!({"session_ref": null}))),
            checkpoint(json!({"step": "transcript"})),
            checkpoint(json!({"step": "claude-code-session-finished", "input_tokens": 1})),
            result(
                "succeeded",
                json!({"result": "complete", "inputs": {"input_tokens": 9}}),
            ),
        ]
        .concat();
        let (ex, out, sent, accepted) = exchange(&stream);
        assert_eq!(out, Ok(Disposition::Completed));
        assert_eq!(accepted, 1);
        assert_eq!(Disposition::Completed.harness_error(), None);
        assert_eq!(Disposition::Completed.code(), "completed");
        assert!(Disposition::Completed.launched());
        assert_eq!(
            ex.capabilities(),
            Some(&Capabilities {
                driver: "claude-code".into(),
                version: "0.15.2".into()
            })
        );
        // Checkpoints keep their data object only.
        assert_eq!(ex.checkpoints().len(), 2);
        assert_eq!(ex.checkpoints()[1]["input_tokens"], 1);
        assert!(!ex.checkpoints()[0].contains_key("effect_id"));
        let kinds: Vec<_> = sent.iter().map(|m| m["type"].as_str().unwrap()).collect();
        assert_eq!(kinds, ["hello", "start", "shutdown"]);
        assert!(sent.iter().all(|m| m["proto"] == PROTO));
        assert_eq!(sent[0]["engine_version"], ENGINE_VERSION);
        let start = &sent[1];
        assert_eq!(
            (&start["effect_id"], &start["attempt_id"]),
            (&json!("e1"), &json!("a1"))
        );
        assert_eq!(start["seat"], "trial");
        let keys: Vec<_> = start["input"]
            .as_object()
            .unwrap()
            .keys()
            .cloned()
            .collect();
        let mut want = [
            "allowed_results",
            "context",
            "feature",
            "phase",
            "result_path",
            "role_path",
            "seat",
            "workdir",
        ];
        want.sort();
        assert_eq!(keys, want);
        assert_eq!(
            start["input"]["allowed_results"],
            json!(["complete", "broken"])
        );
        assert_eq!(start["input"]["role_path"], "/w/charter.md");
        assert_eq!(start["input"]["context"]["goal"], "g");
        let msg_ids: std::collections::BTreeSet<_> = sent
            .iter()
            .map(|m| m["msg_id"].as_str().unwrap().to_string())
            .collect();
        assert_eq!(msg_ids.len(), 3);
    }

    #[test]
    fn a_driver_that_launches_nothing_is_a_harness_error_never_gave_up() {
        // Silent exit zero: not even capabilities, so no start goes out.
        let (_, out, sent, accepted) = exchange("");
        let d = out.unwrap();
        assert_eq!(d.harness_error(), Some("driver_launched_nothing"));
        assert!(!d.launched());
        assert_eq!(accepted, 0);
        let kinds: Vec<_> = sent.iter().map(|m| m["type"].as_str().unwrap()).collect();
        assert_eq!(kinds, ["hello", "shutdown"]);
        // Capabilities, then nothing: start went out, nothing was accepted.
        let (_, out, sent, _) = exchange(&caps());
        assert_eq!(
            out.unwrap().harness_error(),
            Some("driver_launched_nothing")
        );
        assert_eq!(sent.len(), 3);
        // A pre-session refusal: a failed result with no accepted.
        let (_, out, sent, accepted) = exchange(
            &[
                caps(),
                frame(
                    "result",
                    with_ids(
                        json!({"status": "failed", "result": null, "error": "SECRET-PEER-TEXT"}),
                    ),
                ),
            ]
            .concat(),
        );
        let d = out.unwrap();
        assert_eq!(d, Disposition::NotLaunched { refused: true });
        assert_eq!(d.harness_error(), Some("driver_refused_start"));
        assert_eq!(accepted, 0);
        assert_eq!(sent.last().unwrap()["type"], "shutdown");
        assert!(!format!("{d:?}{}", d.code()).contains("SECRET"));
        // Accepted only: launched, its result lost.
        let (_, out, _, accepted) = exchange(&[caps(), frame("accepted", ids())].concat());
        assert_eq!(out, Ok(Disposition::ResultMissing));
        assert_eq!(accepted, 1);
        assert_eq!(
            Disposition::ResultMissing.harness_error(),
            Some("driver_result_missing")
        );
        // Failed after acceptance, and succeeded results with a null, a
        // non-object, a missing or an out-of-vocabulary payload.
        for (res, want) in [
            (result("failed", json!(null)), Disposition::Failed),
            (result("succeeded", json!(null)), Disposition::InvalidResult),
            (
                result("succeeded", json!("complete")),
                Disposition::InvalidResult,
            ),
            (
                result("succeeded", json!({"notes": "n"})),
                Disposition::InvalidResult,
            ),
            (
                result("succeeded", json!({"result": "gave_up"})),
                Disposition::InvalidResult,
            ),
            (
                frame("result", with_ids(json!({"status": "succeeded"}))),
                Disposition::InvalidResult,
            ),
        ] {
            let (_, out, _, _) = exchange(&[caps(), frame("accepted", ids()), res].concat());
            assert_eq!(out, Ok(want));
            assert!(want.launched() && want.harness_error().is_some());
        }
        assert_eq!(Disposition::Failed.code(), "driver_session_failed");
        assert_eq!(Disposition::InvalidResult.code(), "driver_result_invalid");
    }

    #[test]
    fn order_identity_and_trailing_frames_are_refused() {
        let acc = || frame("accepted", ids());
        let res = || result("succeeded", json!({"result": "complete"}));
        let cp = || checkpoint(json!({}));
        let accepting = |extra: Value| [caps(), frame("accepted", extra)].concat();
        let cases: Vec<(String, ProtocolError)> = vec![
            (acc(), ProtocolError::OutOfOrder),
            (cp(), ProtocolError::OutOfOrder),
            (res(), ProtocolError::OutOfOrder),
            ([caps(), caps()].concat(), ProtocolError::OutOfOrder),
            ([caps(), acc(), acc()].concat(), ProtocolError::OutOfOrder),
            ([caps(), cp()].concat(), ProtocolError::OutOfOrder),
            // A succeeded result before accepted is not a refusal.
            ([caps(), res()].concat(), ProtocolError::OutOfOrder),
            (frame("bogus", json!({})), ProtocolError::UnknownType),
            (
                frame("capabilities", json!({"version": "v", "supports": []})),
                ProtocolError::MissingField,
            ),
            (
                frame("capabilities", json!({"driver": "d", "supports": []})),
                ProtocolError::MissingField,
            ),
            (
                frame("capabilities", json!({"driver": "d", "version": "v"})),
                ProtocolError::MissingField,
            ),
            (
                frame(
                    "capabilities",
                    json!({"driver": "d", "version": "v", "supports": [1]}),
                ),
                ProtocolError::MissingField,
            ),
            (
                accepting(json!({"effect_id": "e1"})),
                ProtocolError::MissingField,
            ),
            (
                accepting(json!({"effect_id": "e2", "attempt_id": "a1"})),
                ProtocolError::IdentityMismatch,
            ),
            (
                accepting(json!({"effect_id": "e1", "attempt_id": 1})),
                ProtocolError::IdentityMismatch,
            ),
            (
                [
                    caps(),
                    frame(
                        "result",
                        json!({"effect_id": "x", "attempt_id": "a1", "status": "failed"}),
                    ),
                ]
                .concat(),
                ProtocolError::IdentityMismatch,
            ),
            (
                [
                    caps(),
                    acc(),
                    frame(
                        "checkpoint",
                        json!({"attempt_id": "o", "effect_id": "e1", "data": {}}),
                    ),
                ]
                .concat(),
                ProtocolError::IdentityMismatch,
            ),
            (
                [caps(), acc(), frame("checkpoint", json!({"data": {}}))].concat(),
                ProtocolError::MissingField,
            ),
            (
                [caps(), acc(), frame("checkpoint", ids())].concat(),
                ProtocolError::MissingField,
            ),
            (
                [
                    caps(),
                    acc(),
                    frame("checkpoint", with_ids(json!({"data": [1]}))),
                ]
                .concat(),
                ProtocolError::MissingField,
            ),
            (
                [
                    caps(),
                    acc(),
                    frame("result", json!({"attempt_id": "a1", "status": "failed"})),
                ]
                .concat(),
                ProtocolError::MissingField,
            ),
            (
                [
                    caps(),
                    acc(),
                    frame("result", with_ids(json!({"status": "ok"}))),
                ]
                .concat(),
                ProtocolError::BadStatus,
            ),
            (
                [caps(), acc(), frame("result", ids())].concat(),
                ProtocolError::BadStatus,
            ),
            (
                [caps(), acc(), res(), res()].concat(),
                ProtocolError::AfterResult,
            ),
            (
                [caps(), acc(), res(), cp()].concat(),
                ProtocolError::AfterResult,
            ),
        ];
        for (stream, want) in cases {
            let (_, out, sent, _) = exchange(&stream);
            assert_eq!(out.err(), Some(want), "{stream}");
            // A protocol error still sends nothing after the frames it saw.
            assert!(sent.len() <= 3, "{stream}");
        }
        // A repeated msg_id refuses, whatever the type.
        let one = caps();
        let again = one.replace("capabilities", "accepted");
        assert_eq!(
            exchange(&[one, again].concat()).1.err(),
            Some(ProtocolError::DuplicateMsgId)
        );
        // A frame the decoder never saw still cannot pass for a known type.
        let mut ex = Exchange::new(id(), vec![]);
        assert_eq!(ex.receive(Map::new()), Err(ProtocolError::MissingField));
        let mut m = Map::new();
        m.insert("msg_id".into(), "x".into());
        assert_eq!(ex.receive(m), Err(ProtocolError::UnknownType));
    }

    #[test]
    fn frames_are_decoded_strictly() {
        let cases: Vec<(&[u8], ProtocolError)> =
            vec![
            (b"\xff\xfe", ProtocolError::NotUtf8),
            (b"{", ProtocolError::NotJson),
            (b"{} {}", ProtocolError::NotJson),
            (b"[1]", ProtocolError::NotObject),
            (
                br#"{"proto":"forge-driver/v1","proto":"forge-driver/v1","msg_id":"m","type":"x"}"#,
                ProtocolError::DuplicateKey,
            ),
            (
                br#"{"proto":"forge-driver/v1","msg_id":"m","type":"x","a":{"b":1,"b":2}}"#,
                ProtocolError::DuplicateKey,
            ),
            (br#"{"proto":"forge-driver/v2","msg_id":"m","type":"x"}"#, ProtocolError::WrongProto),
            (br#"{"proto":1,"msg_id":"m","type":"x"}"#, ProtocolError::WrongProto),
            (br#"{"msg_id":"m","type":"x"}"#, ProtocolError::MissingField),
            (br#"{"proto":"forge-driver/v1","type":"x"}"#, ProtocolError::MissingField),
            (br#"{"proto":"forge-driver/v1","msg_id":"m","type":3}"#, ProtocolError::MissingField),
        ];
        for (bytes, want) in cases {
            assert_eq!(
                decode(bytes).err(),
                Some(want),
                "{}",
                String::from_utf8_lossy(bytes)
            );
        }
        // Every scalar kind decodes, and containers nest.
        let ok = decode(
            br#"{"proto":"forge-driver/v1","msg_id":"m","type":"x","v":[true,-1,2,1.5,"s",null,{}]}"#,
        )
        .unwrap();
        assert_eq!(ok["v"], json!([true, -1, 2, 1.5, "s", null, {}]));
        // Depth: the envelope is 1; MAX_DEPTH holds and one more refuses,
        // for arrays and objects alike.
        let nest = |n: usize, open: &str, close: &str| {
            let v = format!("{}1{}", open.repeat(n), close.repeat(n));
            format!(r#"{{"proto":"forge-driver/v1","msg_id":"m","type":"x","v":{v}}}"#)
        };
        for (open, close) in [("[", "]"), (r#"{"k":"#, "}")] {
            assert!(decode(nest(MAX_DEPTH - 1, open, close).as_bytes()).is_ok());
            let deep = nest(MAX_DEPTH, open, close);
            assert_eq!(decode(deep.as_bytes()).err(), Some(ProtocolError::TooDeep));
        }
        // Every error is a fixed code, never the peer's bytes.
        let all = [
            ProtocolError::Io,
            ProtocolError::FrameTooLarge,
            ProtocolError::TotalTooLarge,
            ProtocolError::TooManyFrames,
            ProtocolError::Truncated,
            ProtocolError::NotUtf8,
            ProtocolError::NotJson,
            ProtocolError::NotObject,
            ProtocolError::DuplicateKey,
            ProtocolError::TooDeep,
            ProtocolError::WrongProto,
            ProtocolError::MissingField,
            ProtocolError::UnknownType,
            ProtocolError::OutOfOrder,
            ProtocolError::IdentityMismatch,
            ProtocolError::AfterResult,
            ProtocolError::DuplicateMsgId,
            ProtocolError::BadStatus,
        ];
        let codes: std::collections::BTreeSet<_> = all.iter().map(|e| e.code()).collect();
        assert_eq!(codes.len(), all.len());
        assert!(codes.iter().all(|c| c.starts_with("driver_")));
    }

    /// A frame of exactly `len` bytes before its newline.
    fn sized(len: usize) -> Vec<u8> {
        let head = format!(r#"{{"proto":"{PROTO}","msg_id":"m","type":"capabilities","pad":""#);
        let tail = r#""}"#;
        let mut out = head.into_bytes();
        out.resize(len - tail.len(), b'x');
        out.extend_from_slice(tail.as_bytes());
        out.push(b'\n');
        out
    }

    #[test]
    fn the_reader_holds_its_bounds_at_and_beyond() {
        // At the frame cap, filled in small chunks so the cap is checked
        // across fills; one byte beyond refuses.
        let at = sized(MAX_FRAME_BYTES);
        let mut r = FrameReader::new(std::io::BufReader::with_capacity(7, Cursor::new(at)));
        assert!(r.next_frame().unwrap().is_some());
        assert_eq!(r.next_frame(), Ok(None));
        let mut r = FrameReader::new(Cursor::new(sized(MAX_FRAME_BYTES + 1)));
        assert_eq!(r.next_frame().err(), Some(ProtocolError::FrameTooLarge));
        // Truncation: a last line without its newline.
        let mut r = FrameReader::new(Cursor::new(b"{}".to_vec()));
        assert_eq!(r.next_frame().err(), Some(ProtocolError::Truncated));
        // The aggregate cap: whole frames up to it, the next one refuses.
        let big = sized(MAX_FRAME_BYTES);
        let count = MAX_TOTAL_BYTES / big.len();
        let mut r = FrameReader::new(Cursor::new(big.repeat(count + 1)));
        for _ in 0..count {
            assert!(r.next_frame().unwrap().is_some());
        }
        assert_eq!(r.next_frame().err(), Some(ProtocolError::TotalTooLarge));
        // The frame count: MAX_FRAMES frames read, one more refuses.
        let small = frame("capabilities", json!({})).into_bytes();
        let mut r = FrameReader::new(Cursor::new(small.repeat(MAX_FRAMES + 1)));
        for _ in 0..MAX_FRAMES {
            assert!(r.next_frame().unwrap().is_some());
        }
        assert_eq!(r.next_frame().err(), Some(ProtocolError::TooManyFrames));
    }

    struct Broken;
    impl std::io::Read for Broken {
        fn read(&mut self, _: &mut [u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("x"))
        }
    }

    #[test]
    fn io_failures_are_typed_in_both_directions() {
        let mut r = FrameReader::new(std::io::BufReader::new(Broken));
        assert_eq!(r.next_frame().err(), Some(ProtocolError::Io));
        let full = [
            caps(),
            frame("accepted", ids()),
            result("succeeded", json!({"result": "complete"})),
        ]
        .concat();
        // A send that fails at the hello, at the start and at the shutdown.
        for fail_at in 0..3 {
            let mut n = 0;
            let (_, out) = run(
                Cursor::new(full.clone().into_bytes()),
                id(),
                &input(),
                |_, _| {
                    n += 1;
                    if n > fail_at {
                        Err(std::io::Error::other("closed"))
                    } else {
                        Ok(())
                    }
                },
                || {},
            );
            assert_eq!(out.err(), Some(ProtocolError::Io));
        }
        // A shutdown after EOF that cannot be sent is not an error: the
        // driver is already gone.
        let mut n = 0;
        let (_, out) = run(
            Cursor::new(caps().into_bytes()),
            id(),
            &input(),
            |_| {
                n += 1;
                if n > 2 {
                    Err(std::io::Error::other("closed"))
                } else {
                    Ok(())
                }
            },
            || {},
        );
        assert_eq!(out, Ok(Disposition::NotLaunched { refused: false }));
    }
}
