//! stdio MCP transport: bridge newline-delimited JSON-RPC on stdin/stdout to a
//! running Swarmail's `POST /mcp`. Lets Claude Desktop / Claude Code / Codex
//! launch `swarmail mcp` as a subprocess.
//!
//! The bridge keeps its hand-rolled HTTP/1.1 client (decision 0029 §4): the
//! deadline-expiry arm must stay hermetic — under a client crate the connect
//! timeout cannot be injected, and the test that proves the named error
//! would need a real 65-second sleep. It gains a status-line parse, a body
//! cap, and the rule that stdout carries protocol or nothing: a notification
//! (an id-less object with a string method) and a blank line write nothing,
//! in both the success and the failure arm; every other line is owed the
//! reply the shared classification names (decisions 0044 §1, proposed
//! 0047 §3).
//!
//! The URL's authority is dialed with HTTP's default port 80 when it names
//! none, and sent verbatim as the `Host` header (proposed 0047 §4, issue
//! #133). A request body over the server's own [`MCP_BODY_LIMIT`] is
//! refused locally, never dialed or posted: `-32600` in the server's 413
//! wording, carrying the line's own ids — one per owed reply, the same
//! shape a transport failure gets — so a stdio client can match it.
//!
//! Lines are independent (decision 0039): up to [`BRIDGE_CONCURRENCY`]
//! HTTP exchanges are outstanding at once, so a wait never parks the seed
//! that would satisfy it (issue #90). Replies are written as their exchanges
//! complete — order is promised only inside one batch; a client that needs
//! two mutations ordered awaits the first reply before sending the second.
//! A batch is answered like the server answers it (issue #72): the shared
//! [`crate::mcp::classify`] decides which lines are owed a reply. A line
//! that is not UTF-8 or not JSON gets a local `-32700` and never reaches
//! the network; the bridge keeps reading.
//!
//! Named residuals: a single input line is not length-capped as it is read
//! and parsed — the body limit bounds what is sent, not what is held — so
//! one endless line can exhaust memory; a stdout consumer that stops
//! reading stalls the bridge (and its EOF drain) indefinitely; sixteen
//! parked waits delay a seventeenth line; a transport error means the
//! outcome is unknown, never that a mutation rolled back — nothing is
//! retried; and a server (or proxy) with a lower body limit than this
//! build's still answers its own null-id refusal, forwarded as sent.

use crate::api::{
    MAX_AWAIT_TIMEOUT_MS, MCP_BODY_LIMIT, MCP_BODY_TOO_LARGE, mcp_body_refusal, split_authority,
};
use crate::mcp::{Envelope, classify, error_object};
use futures_util::StreamExt as _;
use futures_util::stream::FuturesUnordered;
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

/// The most HTTP exchanges the bridge keeps outstanding (decision 0039 §4),
/// notifications included. At the bound no further line is read until one
/// completes. Independent of `MCP_BATCH_CAP`, which bounds one batch.
pub const BRIDGE_CONCURRENCY: usize = 16;

/// The dial deadline production uses; a parameter of [`dial`] so the expiry
/// arm is verifiable in-process against a peer that never answers.
const DIAL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// The slack over the await ceiling (decision 0029 §4): the bridge must
/// outlive any wait the server can legally park on, plus enough for the
/// exchange itself — and no more.
pub(crate) const BRIDGE_SLACK: std::time::Duration = std::time::Duration::from_secs(5);

/// The response body cap (decision 0029 §4): 10 MiB. A tool result that
/// big is not a tool result; reading past it would let a peer pin the
/// bridge's memory.
const RESPONSE_CAP: u64 = 10 * 1024 * 1024;

/// The read deadline production uses (decision 0029 §4): derived, not
/// fixed — `MAX_AWAIT_TIMEOUT_MS + BRIDGE_SLACK` — so the one constant
/// that owns the wait ceiling owns the bridge's budget and no future cap
/// change can re-create the old 10-second clamp.
pub fn bridge_deadline() -> std::time::Duration {
    std::time::Duration::from_millis(MAX_AWAIT_TIMEOUT_MS) + BRIDGE_SLACK
}

/// Dial the MCP server: the deadline expiry and the io failure surface as
/// distinct, client-usable error strings.
async fn dial(
    authority: &str,
    deadline: std::time::Duration,
) -> Result<tokio::net::TcpStream, String> {
    tokio::time::timeout(
        deadline,
        tokio::net::TcpStream::connect(authority.to_string()),
    )
    .await
    .map_err(|_| "connect timeout".to_string())?
    .map_err(|e| e.to_string())
}

/// The `host:port` to dial for a URL's authority (proposed 0047 §4, issue
/// #133): an omitted port is HTTP's 80 — `TcpStream::connect` needs one —
/// for a name, an IPv4 address or a bracketed IPv6 literal alike; a port
/// that is spelled is kept, and one that is malformed refuses by name
/// instead of reaching the resolver. The server's own authority parser
/// decides which is which.
fn dial_target(authority: &str) -> Result<String, String> {
    let (host, port) = split_authority(authority)
        .ok_or_else(|| format!("SWARMAIL_URL has a malformed authority: {authority}"))?;
    Ok(format!("{host}:{}", port.unwrap_or(80)))
}

/// What the server sent: protocol to forward, or declared silence.
#[derive(Debug, PartialEq)]
enum Outcome {
    /// A JSON body — forwarded as the same value, written as one compact
    /// line (a multi-line body cannot split the reply).
    Json(Value),
    /// An empty (or whitespace) body: the server declared no content. A
    /// notification's 202 is this, and the bridge writes nothing for it.
    Empty,
}

/// Parse the response: the status line first, then the body the headers
/// declare (decision 0029 §4). A non-2xx carrying a JSON-RPC error object
/// is forwarded verbatim — a server-declared error is a JSON-RPC error;
/// any other non-2xx refuses with the status named. A non-JSON 2xx body
/// refuses rather than fabricating `"{}"`.
fn parse_response(raw: &[u8]) -> Result<Outcome, String> {
    // The cap is judged on what was read: the caller's `take` stops at
    // RESPONSE_CAP, so a full buffer means the reply was at least that big.
    if raw.len() as u64 >= RESPONSE_CAP {
        return Err(format!(
            "response exceeds the {} MiB cap",
            RESPONSE_CAP / 1024 / 1024
        ));
    }
    let sep = raw
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| "malformed response: no header/body separator".to_string())?;
    let head = String::from_utf8_lossy(&raw[..sep]);
    let body = &raw[sep + 4..];
    let status = head
        .lines()
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .and_then(|code| code.parse::<u16>().ok())
        .ok_or_else(|| "malformed response: unreadable status line".to_string())?;
    // content-length is honored when present and plausible; a reply without
    // one (or with a short one) falls back to the raw tail. The client is
    // HTTP/1.1 with identity encoding only: a chunked reply has no
    // content-length and surfaces here, loudly — the named residual.
    let body = match content_length(&head) {
        Some(n) if body.len() >= n => &body[..n],
        _ => body,
    };
    let text = std::str::from_utf8(body).ok();
    let value = text.and_then(|t| serde_json::from_str::<Value>(t).ok());
    if !(200..300).contains(&status) {
        // A non-2xx that still answers a JSON-RPC error object carries the
        // upstream's own refusal — forwarded, not rewrapped (issue #40).
        return value
            .filter(|v| v.get("error").is_some())
            .map(Outcome::Json)
            .ok_or_else(|| format!("server answered {status}"));
    }
    let text = text.ok_or_else(|| "the reply is not valid UTF-8".to_string())?;
    if text.trim().is_empty() {
        return Ok(Outcome::Empty);
    }
    value
        .map(Outcome::Json)
        .ok_or_else(|| format!("the reply is not JSON: {}", text.trim()))
}

/// The `Content-Length` header value of a response head, if any.
fn content_length(head: &str) -> Option<usize> {
    head.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if name.trim().eq_ignore_ascii_case("content-length") {
            value.trim().parse::<usize>().ok()
        } else {
            None
        }
    })
}

/// POST `body` to `{base}/mcp`, returning what the server sent. The
/// deadline is a parameter (decision 0029 §4): production passes the
/// derived bridge deadline, tests a short one so the expiry arm stays
/// hermetic.
async fn rpc_post(
    base: &str,
    body: &str,
    deadline: std::time::Duration,
) -> Result<Outcome, String> {
    let rest = base
        .strip_prefix("http://")
        .ok_or_else(|| "SWARMAIL_URL must be http:// (the local server has no TLS)".to_string())?;
    let (authority, path) = match rest.find('/') {
        Some(i) => (&rest[..i], &rest[i..]),
        None => (rest, ""),
    };
    let mut stream = dial(&dial_target(authority)?, DIAL_TIMEOUT).await?;

    let req = format!(
        "POST {}/mcp HTTP/1.1\r\nHost: {authority}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        path.trim_end_matches('/'),
        body.len()
    );

    // One owner for the transport failure: the request write and the response
    // read surface through the same io-error text, whatever the peer does.
    let mut buf = Vec::new();
    let exchanged = async {
        stream.write_all(req.as_bytes()).await?;
        // The cap is the read's own limit: the reply cannot grow past it.
        (&mut stream).take(RESPONSE_CAP).read_to_end(&mut buf).await
    };
    tokio::time::timeout(deadline, exchanged)
        .await
        .map_err(|_| "read timeout".to_string())?
        .map_err(|e| e.to_string())?;
    parse_response(&buf)
}

/// What one complete input line asks of the bridge (decision 0039 §1): the
/// line is parsed once, from bytes, here — never again for its id.
#[derive(Debug, PartialEq)]
enum Line {
    /// A whitespace-only line: silence, no POST.
    Blank,
    /// Answered here and never posted: a local `-32700` for a line that is
    /// not UTF-8 or not JSON, or the body-limit refusal for one too big to
    /// send — `None` when that oversized line owes no reply.
    Local(Option<String>),
    /// A JSON value to POST, and how its answer is owed.
    Post(String, Envelope),
}

/// Read one line's bytes. Invalid UTF-8 is refused, never repaired with
/// replacement characters; any JSON value — a scalar included — is posted
/// if it fits the server's body limit, so a notification's effect still
/// happens even though it is answered with silence. The limit is judged on
/// the exact bytes that would be posted — the trimmed line, delimiter
/// excluded; equal is sent, one byte over is refused locally (proposed
/// 0047 §4) after the parse, so a malformed line keeps its `-32700`.
fn read_line(bytes: &[u8]) -> Line {
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text.trim(),
        Err(_) => return Line::Local(Some(parse_error("the line is not valid UTF-8"))),
    };
    if text.is_empty() {
        return Line::Blank;
    }
    match serde_json::from_str::<Value>(text) {
        Ok(value) if text.len() > MCP_BODY_LIMIT => Line::Local(refuse(
            &classify(&value),
            -32600,
            &mcp_body_refusal(MCP_BODY_TOO_LARGE),
        )),
        Ok(value) => Line::Post(text.to_string(), classify(&value)),
        Err(e) => Line::Local(Some(parse_error(&e.to_string()))),
    }
}

/// The local parse refusal: the server's own `-32700` envelope, id null.
fn parse_error(why: &str) -> String {
    error_object(-32700, &format!("parse error: {why}"), Value::Null).to_string()
}

/// The reply line an exchange owes (decision 0039 §§2–3), applied locally
/// in both arms so a line-parsing client never reads a reply it did not ask
/// for. A transport failure answers every id the line carried — one array
/// for an admitted batch, in member order — and one null-id error for a
/// batch the server would have refused whole.
fn answer(envelope: &Envelope, outcome: Result<Outcome, String>) -> Option<String> {
    match (envelope, outcome) {
        (Envelope::Silent, _) | (_, Ok(Outcome::Empty)) => None,
        (_, Ok(Outcome::Json(reply))) => Some(reply.to_string()),
        (_, Err(e)) => refuse(envelope, -32000, &e),
    }
}

/// A refusal the bridge itself answers, shaped by the envelope: one error
/// for a single request, one array of errors in member order for an
/// admitted batch (duplicate and null ids kept), one null-id error for a
/// batch the server would refuse whole, and nothing for a notification.
fn refuse(envelope: &Envelope, code: i64, why: &str) -> Option<String> {
    let error = |id: &Value| error_object(code, why, id.clone());
    match envelope {
        Envelope::Silent => None,
        Envelope::Single(id) => Some(error(id).to_string()),
        Envelope::Batch(ids) => Some(Value::Array(ids.iter().map(error).collect()).to_string()),
        Envelope::InvalidBatch => Some(error(&Value::Null).to_string()),
    }
}

/// An outstanding exchange, boxed so the loop can name its collection.
type Exchange<'a> = std::pin::Pin<Box<dyn Future<Output = Option<String>> + Send + 'a>>;

/// One line's exchange: POST it and shape what is owed.
async fn exchange(
    base_url: &str,
    body: String,
    envelope: Envelope,
    deadline: std::time::Duration,
) -> Option<String> {
    answer(&envelope, rpc_post(base_url, &body, deadline).await)
}

/// The one write path (decision 0039 §5): the complete line and its
/// newline offered to a single `write_all`, then flushed. Either failure
/// is the caller's to end on.
async fn emit<W>(stdout: &mut W, reply: String) -> std::io::Result<()>
where
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut line = reply.into_bytes();
    line.push(b'\n');
    stdout.write_all(&line).await?;
    stdout.flush().await
}

pub async fn run_stdio_bridge(base_url: &str) -> i32 {
    bridge(
        base_url,
        &mut BufReader::new(tokio::io::stdin()),
        &mut tokio::io::stdout(),
        bridge_deadline(),
    )
    .await
}

/// The bridge core, generic over reader/writer so tests drive it in-process.
/// The deadline is a parameter: production passes [`bridge_deadline`], tests
/// a short one.
///
/// One loop owns stdout and the outstanding exchanges (decision 0039 §4):
/// completions are taken first, so a reader that is always ready cannot
/// starve them, and the reader is polled only below [`BRIDGE_CONCURRENCY`].
/// The partial line lives outside the loop — `read_until` appends what it
/// consumed before a completion wins the race, so no byte is lost or read
/// twice. EOF drains every admitted exchange and exits 0; a stdin error
/// stops reading, drains, and exits 1; a stdout failure exits 1 at once,
/// dropping what is outstanding (the server may still finish it).
pub async fn bridge<R, W>(
    base_url: &str,
    input: &mut R,
    stdout: &mut W,
    deadline: std::time::Duration,
) -> i32
where
    R: tokio::io::AsyncBufRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut pending: FuturesUnordered<Exchange<'_>> = FuturesUnordered::new();
    let mut partial = Vec::new();
    let mut reading = true;
    let mut status = 0;
    loop {
        let line = tokio::select! {
            biased;
            done = pending.next(), if !pending.is_empty() => {
                if let Some(reply) = done.flatten()
                    && emit(stdout, reply).await.is_err()
                {
                    return 1;
                }
                continue;
            }
            read = input.read_until(b'\n', &mut partial),
                if reading && pending.len() < BRIDGE_CONCURRENCY =>
            {
                match read {
                    Ok(_) if partial.ends_with(b"\n") => std::mem::take(&mut partial),
                    // EOF: whatever is left is the final, unterminated line
                    // (an empty one reads as blank).
                    Ok(0) => {
                        reading = false;
                        std::mem::take(&mut partial)
                    }
                    // EOF reached mid-line: the next read returns 0 and
                    // hands the line over.
                    Ok(_) => continue,
                    Err(e) => {
                        // A fragment cut by the error is never invented
                        // into a request; what was admitted still drains.
                        tracing::warn!(error = %e, "stdio read error");
                        reading = false;
                        status = 1;
                        continue;
                    }
                }
            }
            else => return status,
        };
        match read_line(&line) {
            Line::Blank => {}
            Line::Local(reply) => {
                if let Some(reply) = reply
                    && emit(stdout, reply).await.is_err()
                {
                    return 1;
                }
            }
            Line::Post(body, envelope) => {
                pending.push(Box::pin(exchange(base_url, body, envelope, deadline)));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bridge_deadline_is_the_await_ceiling_plus_the_slack() {
        assert_eq!(
            bridge_deadline(),
            std::time::Duration::from_millis(MAX_AWAIT_TIMEOUT_MS) + BRIDGE_SLACK,
            "the bridge's budget is derived from the await ceiling (decision 0029 §4)"
        );
        assert_eq!(bridge_deadline().as_secs(), 65);
    }

    /// The response parser's arms (decision 0029 §4), in-process: the
    /// status line, the declared body, the forwarded server error, the
    /// content-length slice, the cap and every refusal shape.
    #[test]
    fn the_response_parser_answers_for_every_shape() {
        // 2xx JSON is forwarded; a whitespace-only body is declared silence.
        assert_eq!(
            parse_response(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 9\r\n\r\n{\"id\":1}")
                .unwrap(),
            Outcome::Json(serde_json::json!({ "id": 1 }))
        );
        assert_eq!(
            parse_response(b"HTTP/1.1 202 Accepted\r\nContent-Length: 2\r\n\r\n  ").unwrap(),
            Outcome::Empty
        );
        // A server-declared JSON-RPC error is forwarded verbatim — even on
        // a 400 (axum's parse refusal is exactly this shape).
        let err_body = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32700,"message":"p"}}"#;
        assert_eq!(
            parse_response(
                format!(
                    "HTTP/1.1 400 Bad Request\r\nContent-Length: {}\r\n\r\n{err_body}",
                    err_body.len()
                )
                .as_bytes()
            )
            .unwrap(),
            Outcome::Json(serde_json::from_str(err_body).unwrap())
        );
        // A non-2xx without a JSON-RPC error refuses with the status named.
        assert_eq!(
            parse_response(b"HTTP/1.1 500 Internal Server Error\r\nContent-Length: 5\r\n\r\noops!")
                .unwrap_err(),
            "server answered 500"
        );
        // A 2xx non-JSON body refuses — never a fabricated "{}".
        assert_eq!(
            parse_response(b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello").unwrap_err(),
            "the reply is not JSON: hello"
        );
        // …and so does a body that is not even UTF-8.
        assert_eq!(
            parse_response(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n\xff\xfe").unwrap_err(),
            "the reply is not valid UTF-8"
        );
        // An unreadable status line, and no separator at all.
        assert_eq!(
            parse_response(b"not http at all\r\n\r\n{}").unwrap_err(),
            "malformed response: unreadable status line"
        );
        assert_eq!(
            parse_response(b"HTTP/1.1 200 OK\r\n").unwrap_err(),
            "malformed response: no header/body separator"
        );
        // content-length is honored: a longer tail is not read as body.
        assert_eq!(
            parse_response(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\n{}trailing").unwrap(),
            Outcome::Json(serde_json::json!({}))
        );
        // …a nonsense one is ignored…
        assert_eq!(
            parse_response(b"HTTP/1.1 200 OK\r\nContent-Length: soon\r\n\r\n{}").unwrap(),
            Outcome::Json(serde_json::json!({}))
        );
        // …and a short one falls back to the raw tail (identity encoding
        // only; a chunked reply surfaces here, loudly).
        assert_eq!(
            parse_response(b"HTTP/1.1 200 OK\r\nContent-Length: 9999\r\n\r\n{}").unwrap(),
            Outcome::Json(serde_json::json!({}))
        );
        // The cap: a full take-buffer is a refusal, not a truncation.
        let full = vec![b'x'; RESPONSE_CAP as usize];
        assert!(
            parse_response(&full)
                .unwrap_err()
                .contains("response exceeds the 10 MiB cap")
        );
    }

    /// One line, parsed once from bytes (decision 0039 §1): blank lines are
    /// silence, bad bytes and bad JSON are local `-32700`s with a null id,
    /// and every JSON value — a scalar included — is posted with the shared
    /// classification.
    #[test]
    fn a_line_is_read_once_from_bytes() {
        use serde_json::json;
        assert_eq!(read_line(b" \t\r\n"), Line::Blank);
        assert_eq!(read_line(b""), Line::Blank);
        assert_eq!(
            read_line(b"\xff\n"),
            Line::Local(Some(parse_error("the line is not valid UTF-8")))
        );
        assert_eq!(
            read_line(b"{nope\n"),
            Line::Local(Some(parse_error("key must be a string at line 1 column 2")))
        );
        // The local refusal is the server's own envelope, id null.
        let reply: Value = serde_json::from_str(&parse_error("why")).unwrap();
        assert_eq!(
            reply,
            json!({ "jsonrpc": "2.0", "id": null,
                    "error": { "code": -32700, "message": "parse error: why" } })
        );
        assert_eq!(
            read_line(b"  {\"id\":7,\"method\":\"ping\"}\r\n"),
            Line::Post(
                r#"{"id":7,"method":"ping"}"#.to_string(),
                Envelope::Single(json!(7))
            )
        );
        // A scalar is an invalid request, owed a null-id reply (decision
        // 0044 §1) — still posted, so the server's own answer is forwarded.
        assert_eq!(
            read_line(b"42"),
            Line::Post("42".to_string(), Envelope::Single(Value::Null))
        );
    }

    /// The reply each envelope is owed in both arms (decision 0039 §§2–3):
    /// silence stays silent whatever arrives; a body is forwarded compact;
    /// a failure answers every id the line carried, in order.
    #[test]
    fn every_envelope_is_answered_by_the_table() {
        use serde_json::json;
        let body = || Ok(Outcome::Json(json!({ "id": 1, "result": {} })));
        let failed = || Err("connection refused".to_string());
        let parsed = |reply: Option<String>| -> Value {
            serde_json::from_str(&reply.expect("a reply line")).unwrap()
        };

        assert_eq!(answer(&Envelope::Silent, body()), None);
        assert_eq!(answer(&Envelope::Silent, failed()), None);
        assert_eq!(
            answer(&Envelope::Single(json!(1)), Ok(Outcome::Empty)),
            None
        );
        assert_eq!(
            answer(&Envelope::Single(json!(1)), body()).as_deref(),
            Some(r#"{"id":1,"result":{}}"#)
        );

        let one = parsed(answer(&Envelope::Single(Value::Null), failed()));
        assert_eq!(one["id"], Value::Null, "{one}");
        assert_eq!(one["error"]["code"], -32000, "{one}");
        assert_eq!(one["error"]["message"], "connection refused", "{one}");

        let many = parsed(answer(
            &Envelope::Batch(vec![json!(2), Value::Null, json!(2)]),
            failed(),
        ));
        let ids: Vec<Value> = many
            .as_array()
            .unwrap()
            .iter()
            .map(|e| {
                assert_eq!(e["error"]["code"], -32000, "{e}");
                e["id"].clone()
            })
            .collect();
        assert_eq!(ids, vec![json!(2), Value::Null, json!(2)]);

        let refused = parsed(answer(&Envelope::InvalidBatch, failed()));
        assert_eq!(refused["id"], Value::Null, "{refused}");
        assert_eq!(refused["error"]["code"], -32000, "{refused}");
    }

    /// The dial target (proposed 0047 §4, issue #133; red before: the
    /// authority was dialed as spelled, and a portless one failed with
    /// "invalid socket address"): only an omitted port becomes 80, for a
    /// name, IPv4 and a bracketed IPv6 literal; a spelled port is kept and
    /// a malformed one refuses by name.
    #[test]
    fn bridge_authority_defaults_only_missing_ports() {
        for (authority, target) in [
            ("127.0.0.1", "127.0.0.1:80"),
            ("localhost", "localhost:80"),
            ("[::1]", "[::1]:80"),
            ("127.0.0.1:8025", "127.0.0.1:8025"),
            ("[::1]:9", "[::1]:9"),
            ("localhost:80", "localhost:80"),
        ] {
            assert_eq!(dial_target(authority).as_deref(), Ok(target), "{authority}");
        }
        for malformed in ["localhost:", "localhost:http", "h:65536", "[::1", "[::1]x"] {
            assert_eq!(
                dial_target(malformed),
                Err(format!(
                    "SWARMAIL_URL has a malformed authority: {malformed}"
                )),
            );
        }
    }

    /// A JSON line of exactly `len` bytes: `template` with its `PAD`
    /// replaced by enough copies of `fill`.
    fn sized(template: &str, fill: &str, len: usize) -> String {
        let room = len - (template.len() - "PAD".len());
        assert_eq!(room % fill.len(), 0, "{template} cannot reach {len}");
        template.replace("PAD", &fill.repeat(room / fill.len()))
    }

    /// The local refusal of `code` for `id`, as the line it is written as.
    fn refused(id: Value, code: i64, words: &str) -> Value {
        error_object(code, words, id)
    }

    /// The limit is judged on the exact bytes that would be posted
    /// (proposed 0047 §4, issue #133; red before: every size was posted):
    /// equal is posted, one byte over is a local `-32600` in the server's
    /// own 413 words with the line's id — multibyte text counted in bytes,
    /// the delimiter and padding excluded — and a malformed line keeps its
    /// `-32700` however long it is.
    #[test]
    fn bridge_body_limit_is_byte_exact() {
        let ping = r#"{"jsonrpc":"2.0","id":7,"method":"ping","pad":"PAD"}"#;
        let exact = sized(ping, "x", MCP_BODY_LIMIT);
        let posted = Line::Post(exact.clone(), Envelope::Single(serde_json::json!(7)));
        assert_eq!(read_line(exact.as_bytes()), posted, "equal is posted");
        let padded = format!("  {exact}\r\n");
        assert_eq!(
            read_line(padded.as_bytes()),
            posted,
            "the delimiter is not body"
        );

        let words = mcp_body_refusal(MCP_BODY_TOO_LARGE);
        assert_eq!(
            words,
            "request body refused: Failed to buffer the request body: length limit exceeded \
             (the limit is 2097152 bytes)"
        );
        let want = Line::Local(Some(
            refused(serde_json::json!(7), -32600, &words).to_string(),
        ));
        // One byte over: a one-byte pad, and a two-byte `é` pad whose
        // character count is well under the limit.
        let over_ascii = sized(ping, "x", MCP_BODY_LIMIT + 1);
        let multibyte = sized(ping, "é", MCP_BODY_LIMIT + 1);
        assert!(multibyte.chars().count() < MCP_BODY_LIMIT);
        for over in [over_ascii, multibyte] {
            assert_eq!(over.len(), MCP_BODY_LIMIT + 1);
            assert_eq!(read_line(over.as_bytes()), want);
        }

        let broken = format!("{{{}", "x".repeat(MCP_BODY_LIMIT));
        assert_eq!(
            read_line(broken.as_bytes()),
            Line::Local(Some(parse_error("key must be a string at line 1 column 2"))),
            "the parse refusal comes first"
        );
    }

    /// The local refusal answers every position the server would have
    /// (proposed 0047 §4): one error for a single request, an array in
    /// member order for an admitted batch — string, null and duplicate ids
    /// kept, a malformed member's null slot kept, a notification's omitted
    /// — one null-id error for a batch refused whole, and silence for a
    /// notification. Every code is `-32600`.
    #[test]
    fn local_refusal_preserves_envelope_positions() {
        let pad = |member: &str| member.replace("PAD", &"x".repeat(MCP_BODY_LIMIT));
        use serde_json::json;
        let words = mcp_body_refusal(MCP_BODY_TOO_LARGE);
        let notification = pad(r#"{"jsonrpc":"2.0","method":"tools/call","x":"PAD"}"#);
        assert_eq!(
            read_line(notification.as_bytes()),
            Line::Local(None),
            "a notification stays silent"
        );

        let batch = pad(
            r#"[{"id":7,"method":"ping","x":"PAD"},{"id":"a","method":"ping"},
               {"id":null,"method":"ping"},{"method":"n"},{"foo":"boo"},{"id":7,"method":"ping"}]"#,
        );
        let positions = [json!(7), json!("a"), Value::Null, Value::Null, json!(7)];
        let replies: Vec<Value> = positions
            .into_iter()
            .map(|id| refused(id, -32600, &words))
            .collect();
        assert_eq!(
            read_line(batch.as_bytes()),
            Line::Local(Some(Value::Array(replies).to_string()))
        );

        let whole = pad(&format!(
            "[{}]",
            vec![r#"{"id":1,"method":"ping","x":"PAD"}"#; 17].join(",")
        ));
        assert_eq!(
            read_line(whole.as_bytes()),
            Line::Local(Some(refused(Value::Null, -32600, &words).to_string())),
            "a batch refused whole gets one null-id error"
        );
    }

    /// A refused line is never dialed (proposed 0047 §4): the bridge runs
    /// oversized lines and then one small request against a listener
    /// nobody accepts from. Its accept queue then holds exactly the small
    /// request's connection — the positive control that the queue sees a
    /// dial — and the refusals are written first, in input order.
    #[tokio::test]
    async fn an_oversized_line_is_never_dialed() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let big = "x".repeat(MCP_BODY_LIMIT);
        let input = format!(
            "{{\"id\":7,\"method\":\"ping\",\"x\":\"{big}\"}}\n\
             {{\"method\":\"tools/call\",\"x\":\"{big}\"}}\n\
             {{\"id\":8,\"method\":\"ping\"}}\n"
        );
        let mut out = Vec::new();
        let status = bridge(
            &url,
            &mut input.as_bytes(),
            &mut out,
            std::time::Duration::from_millis(300),
        )
        .await;
        assert_eq!(status, 0);
        let lines: Vec<Value> = String::from_utf8(out)
            .unwrap()
            .lines()
            .map(|l| serde_json::from_str(l).unwrap())
            .collect();
        assert_eq!(
            lines,
            [
                refused(
                    serde_json::json!(7),
                    -32600,
                    &mcp_body_refusal(MCP_BODY_TOO_LARGE)
                ),
                refused(serde_json::json!(8), -32000, "read timeout"),
            ]
        );
        listener.accept().expect("the small request dialed");
        assert_eq!(
            listener.accept().unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock,
            "no oversized line dialed"
        );
    }
}

/// The connect that outlives its deadline is a named error, not a hang
/// (decision 0029 §4): the deadline expiry and the io failure are
/// distinct, client-usable strings. The hang is real — a listener whose
/// accept queue is filled with unaccepted connections, so the kernel
/// drops the next SYN and the connect blocks until the deadline ends it.
#[tokio::test]
async fn a_connect_past_the_deadline_names_the_timeout() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().unwrap().port();
    // Fill the accept queue: connect until a connect stops completing
    // (129 on this kernel's default backlog), then the next one hangs.
    let mut held = Vec::new();
    for _ in 0..1024 {
        match tokio::time::timeout(
            std::time::Duration::from_millis(300),
            tokio::net::TcpStream::connect(("127.0.0.1", port)),
        )
        .await
        {
            Ok(Ok(s)) => held.push(s),
            _ => break,
        }
    }
    assert!(
        !held.is_empty(),
        "the queue must be fillable for this test to mean anything"
    );
    let err = dial(
        &format!("127.0.0.1:{port}"),
        std::time::Duration::from_millis(400),
    )
    .await
    .expect_err("the queued-over connect must not succeed");
    assert_eq!(err, "connect timeout", "{err}");
    let again = dial(
        &format!("127.0.0.1:{port}"),
        std::time::Duration::from_millis(400),
    )
    .await
    .expect_err("the deadline, not the kernel, ends every later attempt");
    assert_eq!(again, "connect timeout", "{again}");
}
