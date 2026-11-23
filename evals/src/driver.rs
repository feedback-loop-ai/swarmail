//! The caller side of brokkr's forge-driver/v1 stdio protocol (decision 0119's
//! 2026-11-23 addendum, route (b); proposed 0138; specs/nested-driver-usage).
//!
//! `swarmail-evals` acts as the nested driver's engine: it sends `hello`, then
//! `start{effect_id, attempt_id, seat, input}` once `capabilities` arrives, then
//! `shutdown`; the driver answers `capabilities`, `accepted`, any number of
//! `checkpoint`, then one `result`. This module holds only what the addendum
//! records: the envelope, the order, the identities, the eight start keys and
//! a bounded strict frame reader. It reads no usage field: which checkpoint
//! or result representation is a family's session-final total needs the
//! release evidence the plan names, so token facts are not derived here yet.
//! No production path launches a driver while `driver_contract_unknown` is
//! refused; this is the client that refusal will be lifted onto.

use serde::de::{DeserializeSeed, Error as _, MapAccess, SeqAccess, Visitor};
use serde_json::{Map, Value};
use std::fmt;
use std::io::BufRead;

/// The protocol name every frame carries in `proto`.
pub const PROTO: &str = "forge-driver/v1";
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
        }
    }
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
    line(envelope("hello", msg_id))
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

/// What the driver did, as far as its frames prove.
#[derive(Debug, Clone, PartialEq)]
pub enum Disposition {
    /// The driver reached no `accepted`: nothing was launched. A driver
    /// that exits 0 here is a harness error, never the agent giving up.
    NotLaunched { refused: Option<String> },
    /// Accepted, but the stream ended before its `result`.
    ResultMissing,
    /// The one `result`, with its `status` when the driver sent one.
    Completed { status: Option<String> },
}

impl Disposition {
    /// The trial's harness-error reason, `None` only for a completed session.
    pub fn harness_error(&self) -> Option<&'static str> {
        match self {
            Self::NotLaunched { refused: None } => Some("driver_launched_nothing"),
            Self::NotLaunched { refused: Some(_) } => Some("driver_refused_start"),
            Self::ResultMissing => Some("driver_result_missing"),
            Self::Completed { .. } => None,
        }
    }
}

fn status(m: &Map<String, Value>) -> Option<String> {
    m.get("status").and_then(Value::as_str).map(str::to_string)
}

/// The ordered exchange's checker: feed it every driver frame; it says when
/// `start` and `shutdown` are due and what the driver did at end of stream.
pub struct Exchange {
    id: Identity,
    stage: Stage,
    checkpoints: Vec<Map<String, Value>>,
    result: Option<Map<String, Value>>,
    refused: Option<String>,
}

impl Exchange {
    pub fn new(id: Identity) -> Self {
        Self {
            id,
            stage: Stage::AwaitCapabilities,
            checkpoints: Vec::new(),
            result: None,
            refused: None,
        }
    }

    /// True once `capabilities` arrived and `start` may be sent.
    pub fn ready_to_start(&self) -> bool {
        self.stage == Stage::AwaitAccepted
    }

    /// True once the `result` arrived and `shutdown` may be sent.
    pub fn finished(&self) -> bool {
        self.stage == Stage::Finished
    }

    /// The checkpoints in order, kept for the usage reader the release
    /// evidence will settle.
    pub fn checkpoints(&self) -> &[Map<String, Value>] {
        &self.checkpoints
    }

    /// The `result` frame of a launched session.
    pub fn result(&self) -> Option<&Map<String, Value>> {
        self.result.as_ref()
    }

    fn check_identity(&self, m: &Map<String, Value>, required: bool) -> Result<(), ProtocolError> {
        for (key, want) in [
            ("effect_id", &self.id.effect_id),
            ("attempt_id", &self.id.attempt_id),
        ] {
            match m.get(key) {
                Some(Value::String(got)) if got == want => {}
                None if !required => {}
                None => return Err(ProtocolError::MissingField),
                Some(_) => return Err(ProtocolError::IdentityMismatch),
            }
        }
        Ok(())
    }

    /// One decoded driver frame.
    pub fn receive(&mut self, m: Map<String, Value>) -> Result<(), ProtocolError> {
        if self.stage == Stage::Finished {
            return Err(ProtocolError::AfterResult);
        }
        let kind = m.get("type").and_then(Value::as_str).unwrap_or_default();
        match (kind, self.stage) {
            ("capabilities", Stage::AwaitCapabilities) => self.stage = Stage::AwaitAccepted,
            ("accepted", Stage::AwaitAccepted) => {
                self.check_identity(&m, true)?;
                self.stage = Stage::Running;
            }
            ("checkpoint", Stage::Running) => {
                self.check_identity(&m, false)?;
                self.checkpoints.push(m);
            }
            ("result", Stage::AwaitAccepted) => {
                // A pre-session refusal: the driver answered start with its
                // result and launched nothing. Keep the status only.
                self.check_identity(&m, true)?;
                self.refused = Some(status(&m).unwrap_or_else(|| "unstated".into()));
                self.stage = Stage::Finished;
            }
            ("result", Stage::Running) => {
                self.check_identity(&m, true)?;
                self.result = Some(m);
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
        match (&self.result, self.stage) {
            (Some(r), _) => Disposition::Completed { status: status(r) },
            (None, Stage::Running) => Disposition::ResultMissing,
            (None, _) => Disposition::NotLaunched {
                refused: self.refused.clone(),
            },
        }
    }
}

/// Drive a whole exchange over one reader: `hello` first, `start` once the
/// capabilities arrive, `shutdown` after the result, every frame decoded and
/// checked in order. Deadlines and the process belong to the caller.
pub fn run<R: BufRead>(
    reader: R,
    id: Identity,
    input: &StartInput,
    mut send: impl FnMut(Vec<u8>) -> std::io::Result<()>,
) -> Result<(Exchange, Disposition), ProtocolError> {
    let mut seq = 0u32;
    let mut next_id = || {
        seq += 1;
        format!("swarmail-evals-{seq}")
    };
    send(hello(&next_id())).map_err(|_| ProtocolError::Io)?;
    let mut ex = Exchange::new(id);
    let mut frames = FrameReader::new(reader);
    let (mut started, mut shut) = (false, false);
    while let Some(frame) = frames.next_frame()? {
        ex.receive(frame)?;
        if ex.ready_to_start() && !started {
            started = true;
            send(start(&next_id(), &ex.id, input)).map_err(|_| ProtocolError::Io)?;
        }
        if ex.finished() && !shut {
            shut = true;
            send(shutdown(&next_id())).map_err(|_| ProtocolError::Io)?;
        }
    }
    let disposition = ex.finish();
    Ok((ex, disposition))
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
        let mut v = json!({"proto": PROTO, "msg_id": "m", "type": kind});
        for (k, x) in extra.as_object().unwrap() {
            v[k] = x.clone();
        }
        format!("{v}\n")
    }

    fn ids() -> Value {
        json!({"effect_id": "e1", "attempt_id": "a1"})
    }

    type Outcome = Result<(Exchange, Disposition), ProtocolError>;

    fn exchange(stream: &str) -> (Outcome, Vec<Value>) {
        let mut sent = Vec::new();
        let out = run(
            Cursor::new(stream.as_bytes().to_vec()),
            id(),
            &input(),
            |b| {
                assert_eq!(b.last(), Some(&b'\n'));
                sent.push(serde_json::from_slice(&b).unwrap());
                Ok(())
            },
        );
        (out, sent)
    }

    #[test]
    fn the_whole_exchange_runs_in_order_and_sends_exactly_the_eight_keys() {
        let stream = [
            frame("capabilities", json!({"driver": "d", "supports": []})),
            frame("accepted", ids()),
            frame("checkpoint", ids()),
            frame("checkpoint", json!({})),
            frame(
                "result",
                json!({"effect_id": "e1", "attempt_id": "a1", "status": "ok"}),
            ),
        ]
        .concat();
        let (out, sent) = exchange(&stream);
        let (ex, disposition) = out.unwrap();
        assert_eq!(
            disposition,
            Disposition::Completed {
                status: Some("ok".into())
            }
        );
        assert_eq!(disposition.harness_error(), None);
        assert_eq!(ex.checkpoints().len(), 2);
        assert!(ex.result().is_some());
        let kinds: Vec<_> = sent.iter().map(|m| m["type"].as_str().unwrap()).collect();
        assert_eq!(kinds, ["hello", "start", "shutdown"]);
        assert!(sent.iter().all(|m| m["proto"] == PROTO));
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
        let (out, sent) = exchange("");
        assert_eq!(
            out.unwrap().1.harness_error(),
            Some("driver_launched_nothing")
        );
        assert_eq!(sent.len(), 1);
        // Capabilities, then nothing: start went out, nothing was accepted.
        let (out, sent) = exchange(&frame("capabilities", json!({})));
        assert_eq!(
            out.unwrap().1.harness_error(),
            Some("driver_launched_nothing")
        );
        assert_eq!(sent.len(), 2);
        // A pre-session refusal, with and without a status.
        for (extra, status) in [
            (
                json!({"effect_id": "e1", "attempt_id": "a1", "status": "failed"}),
                "failed",
            ),
            (ids(), "unstated"),
        ] {
            let stream = [frame("capabilities", json!({})), frame("result", extra)].concat();
            let (out, sent) = exchange(&stream);
            let (ex, d) = out.unwrap();
            assert_eq!(
                d,
                Disposition::NotLaunched {
                    refused: Some(status.into())
                }
            );
            assert_eq!(d.harness_error(), Some("driver_refused_start"));
            assert!(ex.result().is_none());
            assert_eq!(sent.last().unwrap()["type"], "shutdown");
        }
        // Accepted only.
        let stream = [frame("capabilities", json!({})), frame("accepted", ids())].concat();
        let d = exchange(&stream).0.unwrap().1;
        assert_eq!(d, Disposition::ResultMissing);
        assert_eq!(d.harness_error(), Some("driver_result_missing"));
        // A result without a status completes, its status unstated.
        let stream = [
            frame("capabilities", json!({})),
            frame("accepted", ids()),
            frame("result", ids()),
        ]
        .concat();
        assert_eq!(
            exchange(&stream).0.unwrap().1,
            Disposition::Completed { status: None }
        );
    }

    #[test]
    fn order_identity_and_trailing_frames_are_refused() {
        let caps = frame("capabilities", json!({}));
        let acc = frame("accepted", ids());
        let res = frame("result", ids());
        let cp = frame("checkpoint", ids());
        let with = |extra: Value| [caps.clone(), frame("accepted", extra)].concat();
        let cases: Vec<(String, ProtocolError)> = vec![
            (acc.clone(), ProtocolError::OutOfOrder),
            (cp.clone(), ProtocolError::OutOfOrder),
            (res.clone(), ProtocolError::OutOfOrder),
            (
                [caps.clone(), caps.clone()].concat(),
                ProtocolError::OutOfOrder,
            ),
            (
                [caps.clone(), acc.clone(), acc.clone()].concat(),
                ProtocolError::OutOfOrder,
            ),
            (
                [caps.clone(), cp.clone()].concat(),
                ProtocolError::OutOfOrder,
            ),
            (frame("bogus", json!({})), ProtocolError::UnknownType),
            (
                with(json!({"effect_id": "e1"})),
                ProtocolError::MissingField,
            ),
            (
                with(json!({"effect_id": "e2", "attempt_id": "a1"})),
                ProtocolError::IdentityMismatch,
            ),
            (
                with(json!({"effect_id": "e1", "attempt_id": 1})),
                ProtocolError::IdentityMismatch,
            ),
            (
                [
                    caps.clone(),
                    frame("result", json!({"effect_id": "x", "attempt_id": "a1"})),
                ]
                .concat(),
                ProtocolError::IdentityMismatch,
            ),
            (
                [
                    caps.clone(),
                    acc.clone(),
                    frame("checkpoint", json!({"attempt_id": "o"})),
                ]
                .concat(),
                ProtocolError::IdentityMismatch,
            ),
            (
                [
                    caps.clone(),
                    acc.clone(),
                    frame("result", json!({"attempt_id": "a1"})),
                ]
                .concat(),
                ProtocolError::MissingField,
            ),
            (
                [caps.clone(), acc.clone(), res.clone(), res.clone()].concat(),
                ProtocolError::AfterResult,
            ),
            (
                [caps.clone(), acc.clone(), res.clone(), cp.clone()].concat(),
                ProtocolError::AfterResult,
            ),
        ];
        for (stream, want) in cases {
            assert_eq!(exchange(&stream).0.err(), Some(want), "{stream}");
        }
        // A frame the decoder never saw still cannot pass for a known type.
        let mut ex = Exchange::new(id());
        assert_eq!(ex.receive(Map::new()), Err(ProtocolError::UnknownType));
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
            frame("capabilities", json!({})),
            frame("accepted", ids()),
            frame("result", ids()),
        ]
        .concat();
        // A send that fails at the hello, at the start and at the shutdown.
        for fail_at in 0..3 {
            let mut n = 0;
            let out = run(
                Cursor::new(full.clone().into_bytes()),
                id(),
                &input(),
                |_| {
                    n += 1;
                    if n > fail_at {
                        Err(std::io::Error::other("closed"))
                    } else {
                        Ok(())
                    }
                },
            );
            assert_eq!(out.err(), Some(ProtocolError::Io));
        }
    }
}
