//! stdio MCP transport: bridge newline-delimited JSON-RPC on stdin/stdout to a
//! running Swarmail's `POST /mcp`. Lets Claude Desktop / Claude Code / Codex
//! launch `swarmail mcp` as a subprocess.
//!
//! The bridge keeps its hand-rolled HTTP/1.1 client (decision 0029 §4): the
//! deadline-expiry arm must stay hermetic — under a client crate the connect
//! timeout cannot be injected, and the test that proves the named error
//! would need a real 65-second sleep. It gains a status-line parse, a body
//! cap, and the rule that stdout carries protocol or nothing: a notification
//! (and any line that is not a request) writes nothing, in both the success
//! and the failure arm.

use crate::api::MAX_AWAIT_TIMEOUT_MS;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

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

/// What the server sent: protocol to forward, or declared silence.
#[derive(Debug, PartialEq, Eq)]
enum Outcome {
    /// A JSON body — forward verbatim.
    Json(String),
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
    if !(200..300).contains(&status) {
        // A non-2xx that still answers a JSON-RPC error object carries the
        // upstream's own refusal — forwarded, not rewrapped (issue #40).
        if let Some(text) = text.filter(|t| {
            serde_json::from_str::<serde_json::Value>(t)
                .ok()
                .is_some_and(|v| v.get("error").is_some())
        }) {
            return Ok(Outcome::Json(text.to_string()));
        }
        return Err(format!("server answered {status}"));
    }
    let text = text.ok_or_else(|| "the reply is not valid UTF-8".to_string())?;
    if text.trim().is_empty() {
        return Ok(Outcome::Empty);
    }
    if serde_json::from_str::<serde_json::Value>(text).is_err() {
        return Err(format!("the reply is not JSON: {}", text.trim()));
    }
    Ok(Outcome::Json(text.to_string()))
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
    let mut stream = dial(authority, DIAL_TIMEOUT).await?;

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

/// The request id, parsed once per line (decision 0029 §4): `Some` only
/// when the line *is* a request — JSON carrying an `id` key, null
/// included, exactly the predicate the server's envelope answers. Both
/// arms of the bridge read it, so a notification (or any line that is not
/// a request) writes nothing whatever the POST answers.
fn request_id(line: &str) -> Option<serde_json::Value> {
    let req = serde_json::from_str::<serde_json::Value>(line).ok()?;
    if req.get("id").is_some() {
        Some(req["id"].clone())
    } else {
        None
    }
}

pub async fn run_stdio_bridge(base_url: &str) -> i32 {
    bridge(
        base_url,
        &mut BufReader::new(tokio::io::stdin()).lines(),
        &mut tokio::io::stdout(),
        bridge_deadline(),
    )
    .await
}

/// The bridge core, generic over reader/writer so tests drive it in-process.
/// The deadline is a parameter: production passes [`bridge_deadline`], tests
/// a short one.
pub async fn bridge<R, W>(
    base_url: &str,
    lines: &mut tokio::io::Lines<R>,
    stdout: &mut W,
    deadline: std::time::Duration,
) -> i32
where
    R: tokio::io::AsyncBufRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    loop {
        match lines.next_line().await {
            Ok(None) => return 0, // EOF
            Ok(Some(line)) => {
                if line.trim().is_empty() {
                    continue;
                }
                // The predicate is applied locally, not delegated to the
                // server: only a line that is a request can produce an
                // output line, so a line-parsing client never reads a reply
                // it did not ask for (decision 0029 §4).
                let id = request_id(&line);
                match rpc_post(base_url, &line, deadline).await {
                    Ok(Outcome::Json(resp)) => {
                        // The id the bridge parsed is already the id the
                        // server echoed (the envelope rule): forward the
                        // body verbatim, no rewriting.
                        if id.is_some() {
                            let _ = stdout.write_all(resp.as_bytes()).await;
                            let _ = stdout.write_all(b"\n").await;
                            let _ = stdout.flush().await;
                        }
                    }
                    // Declared silence: nothing on stdout, whatever the id.
                    Ok(Outcome::Empty) => {}
                    Err(e) => {
                        // Reply as a JSON-RPC error so the client stays
                        // usable — but a notification stays silent even
                        // when its POST fails.
                        if let Some(id) = id {
                            let err = serde_json::json!({
                                "jsonrpc": "2.0", "id": id,
                                "error": { "code": -32000, "message": e }
                            });
                            let _ = stdout.write_all(err.to_string().as_bytes()).await;
                            let _ = stdout.write_all(b"\n").await;
                            let _ = stdout.flush().await;
                        }
                    }
                }
            }
            Err(e) => {
                tracing::warn!(error = %e, "stdio read error");
                return 1;
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
            Outcome::Json("{\"id\":1}".to_string())
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
            Outcome::Json(err_body.to_string())
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
            Outcome::Json("{}".to_string())
        );
        // …a nonsense one is ignored…
        assert_eq!(
            parse_response(b"HTTP/1.1 200 OK\r\nContent-Length: soon\r\n\r\n{}").unwrap(),
            Outcome::Json("{}".to_string())
        );
        // …and a short one falls back to the raw tail (identity encoding
        // only; a chunked reply surfaces here, loudly).
        assert_eq!(
            parse_response(b"HTTP/1.1 200 OK\r\nContent-Length: 9999\r\n\r\n{}").unwrap(),
            Outcome::Json("{}".to_string())
        );
        // The cap: a full take-buffer is a refusal, not a truncation.
        let full = vec![b'x'; RESPONSE_CAP as usize];
        assert!(
            parse_response(&full)
                .unwrap_err()
                .contains("response exceeds the 10 MiB cap")
        );
    }

    /// The request-id predicate (decision 0029 §4): a request is JSON
    /// carrying an `id` — null included; a notification, a JSON scalar and
    /// a non-JSON line are not requests.
    #[test]
    fn the_request_id_predicate_is_the_envelopes_own() {
        assert_eq!(
            request_id(r#"{"jsonrpc":"2.0","id":7,"method":"ping"}"#),
            Some(serde_json::json!(7))
        );
        assert_eq!(
            request_id(r#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#),
            Some(serde_json::Value::Null)
        );
        assert_eq!(
            request_id(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#),
            None
        );
        assert_eq!(request_id(r#"[1,2,3]"#), None);
        assert_eq!(request_id("42"), None);
        assert_eq!(request_id("{nope"), None);
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
