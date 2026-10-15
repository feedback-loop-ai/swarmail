//! The stdio JSON-RPC bridge, driven in-process over duplex pipes.

use std::time::Duration;
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, duplex};

/// Drive the bridge with two duplex pipes and collect its stdout.
/// Input is fed and closed (EOF), the bridge's writes are read back.
/// The default deadline is comfortable but bounded: every request here is
/// answered in milliseconds — only the named tests pass another value.
async fn bridge_over(base_url: &str, input: &[u8]) -> (i32, String) {
    bridge_over_with_deadline(base_url, input, Duration::from_secs(10)).await
}

/// The same harness with the bridge deadline made explicit — the parameter
/// production derives from the await ceiling (decision 0029 §4).
async fn bridge_over_with_deadline(
    base_url: &str,
    input: &[u8],
    deadline: Duration,
) -> (i32, String) {
    // Pipe 1: test writes input; the bridge reads it (until EOF).
    let (mut input_end, bridge_input) = duplex(64 * 1024);
    // Pipe 2: the bridge writes; the test reads.
    let (mut bridge_output, mut output_end) = duplex(64 * 1024);

    input_end.write_all(input).await.unwrap();
    drop(input_end); // EOF for the bridge's reader

    let (r, w) = tokio::io::split(bridge_input);
    drop(w); // the bridge's end is read-only
    let mut lines = tokio::io::BufReader::new(r).lines();
    let code = swarmail::stdio::bridge(base_url, &mut lines, &mut bridge_output, deadline).await;
    drop(bridge_output); // EOF for the test's reader

    let mut collected = Vec::new();
    let _ = tokio::time::timeout(
        Duration::from_secs(5),
        output_end.read_to_end(&mut collected),
    )
    .await
    .expect("bridge output never ended");
    (code, String::from_utf8_lossy(&collected).to_string())
}

mod common {
    use swarmail::RunningServer;
    pub async fn start() -> RunningServer {
        let cfg = swarmail::config::Config {
            smtp_listen: "127.0.0.1:0".into(),
            http_listen: "127.0.0.1:0".into(),
            pop3_listen: "127.0.0.1:0".into(),
            max_per_inbox: 0,
            data_file: None,
            tls_cert: None,
            tls_key: None,
            smtp: Default::default(),
        };
        swarmail::run_on(&cfg).await.unwrap()
    }
}

#[tokio::test]
async fn bridge_forwards_jsonrpc_and_exits_zero_at_eof() {
    let srv = common::start().await;
    let base = format!("http://{}", srv.http_addr);
    // The trailing blank line is the bridge's own skip: nothing is written
    // for it (the test's old blank-line filter is gone — it was hiding the
    // defect the other way, decision 0029 §4).
    let input = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n\n";
    let (code, out) = bridge_over(&base, input).await;
    assert_eq!(code, 0);
    let responses: Vec<serde_json::Value> = out
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(
        responses.len(),
        1,
        "the blank line produced no output line: {out:?}"
    );
    assert_eq!(responses[0]["result"]["protocolVersion"], "2025-03-26");
}

#[tokio::test]
async fn bridge_replies_jsonrpc_error_when_the_server_is_unreachable() {
    // Port 1 on loopback refuses instantly; the error becomes a JSON-RPC
    // error with the request's id preserved.
    let input = b"{\"jsonrpc\":\"2.0\",\"id\":7,\"method\":\"tools/list\"}\n";
    let (code, out) = bridge_over("http://127.0.0.1:1", input).await;
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(v["id"], 7);
    assert_eq!(v["error"]["code"], -32000);
}

#[tokio::test]
async fn bridge_posts_under_a_base_url_path_prefix() {
    // A base URL with a path keeps the prefix: POST {path}/mcp. Port 1
    // refuses, so the parse (not the reply) is what this proves.
    let input = b"{\"jsonrpc\":\"2.0\",\"id\":9,\"method\":\"tools/list\"}\n";
    let (code, out) = bridge_over("http://127.0.0.1:1/api/v2", input).await;
    assert_eq!(code, 0);
    let v: serde_json::Value = serde_json::from_str(out.trim()).unwrap();
    assert_eq!(v["error"]["code"], -32000);
}

/// A base URL whose path ends in a slash joins slash-safely (decision 0029
/// §4): `//mcp` would 404, `/mcp` answers the request.
#[tokio::test]
async fn a_trailing_slash_base_still_reaches_the_endpoint() {
    let srv = common::start().await;
    let base = format!("http://{}/", srv.http_addr);
    let (code, out) = bridge_over(
        &base,
        br#"{"jsonrpc":"2.0","id":3,"method":"initialize","params":{}}"#,
    )
    .await;
    assert_eq!(code, 0);
    let reply: serde_json::Value = serde_json::from_str(out.trim()).expect("one reply line");
    assert_eq!(reply["id"], 3, "{reply}");
    assert_eq!(
        reply["result"]["protocolVersion"], "2025-03-26",
        "the slash-safe join reached /mcp: {reply}"
    );
}

#[tokio::test]
async fn bridge_reports_read_errors_with_exit_one() {
    struct Boom;
    impl tokio::io::AsyncRead for Boom {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            _cx: &mut std::task::Context<'_>,
            _buf: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::task::Poll::Ready(Err(std::io::Error::other("boom")))
        }
    }
    let mut lines = tokio::io::BufReader::new(Boom).lines();
    let mut sink = duplex(1024).0;
    let code = swarmail::stdio::bridge(
        "http://127.0.0.1:1",
        &mut lines,
        &mut sink,
        Duration::from_secs(10),
    )
    .await;
    assert_eq!(code, 1);
}

/// A base URL that is not http:// refuses every request before dialing:
/// the strip-prefix arm answers as a JSON-RPC error so the client survives.
#[tokio::test]
async fn non_http_base_answers_as_a_jsonrpc_error() {
    let (code, out) = bridge_over(
        "https://x.io",
        br#"{"jsonrpc":"2.0","id":7,"method":"initialize","params":{}}"#,
    )
    .await;
    assert_eq!(code, 0, "EOF is a clean exit");
    let reply: serde_json::Value = serde_json::from_str(out.trim()).expect("one reply line");
    assert_eq!(reply["id"], 7);
    assert_eq!(reply["error"]["code"], -32000);
    assert!(
        reply["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("SWARMAIL_URL must be http://"),
        "{reply}"
    );
}

/// A base whose TCP connect is refused: the error text is the io error, and
/// the bridge keeps serving the next line.
#[tokio::test]
async fn refused_connect_answers_as_a_jsonrpc_error() {
    // Port 1 on localhost: nothing listens there in any test environment.
    let (code, out) = bridge_over(
        "http://127.0.0.1:1",
        br#"{"jsonrpc":"2.0","id":9,"method":"initialize","params":{}}"#,
    )
    .await;
    assert_eq!(code, 0);
    let reply: serde_json::Value = serde_json::from_str(out.trim()).expect("one reply line");
    assert_eq!(reply["error"]["code"], -32000);
    assert!(
        !reply["error"]["message"].as_str().unwrap().is_empty(),
        "{reply}"
    );
}

/// A peer that resets the connection: the request write may land, but the
/// read dies with ECONNRESET and the bridge answers the JSON-RPC error.
#[tokio::test]
async fn reset_peer_answers_as_a_jsonrpc_error() {
    // Accept, then drop the stream with SO_LINGER 0 so the kernel sends a
    // hard RST instead of a graceful FIN.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let stream = listener.accept().await.unwrap().0;
        // The RST is the point: linger-zero makes the drop abort, not FIN.
        #[allow(deprecated)]
        stream.set_linger(Some(Duration::ZERO)).unwrap();
        drop(stream); // RST, not a graceful FIN
        tokio::time::sleep(Duration::from_secs(30)).await;
    });

    let (code, out) = bridge_over(
        &format!("http://{addr}"),
        br#"{"jsonrpc":"2.0","id":13,"method":"initialize","params":{}}"#,
    )
    .await;
    assert_eq!(code, 0);
    let reply: serde_json::Value = serde_json::from_str(out.trim()).expect("one reply line");
    assert_eq!(reply["error"]["code"], -32000, "{reply}");
}

/// A peer that accepts but never answers: the bridge's *derived* deadline
/// fires — here injected short, so the expiry arm stays hermetic (0020's
/// spine) while production's budget is `MAX_AWAIT_TIMEOUT_MS + slack`,
/// pinned by `the_bridge_deadline_is_the_await_ceiling_plus_the_slack`.
#[tokio::test]
async fn a_silent_server_answers_as_a_read_timeout() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        // Accept and HOLD the stream, never reading or writing: the request
        // starves until the injected deadline ends the wait.
        let (_held, _) = listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_secs(5)).await;
    });

    let (code, out) = bridge_over_with_deadline(
        &format!("http://{addr}"),
        br#"{"jsonrpc":"2.0","id":11,"method":"initialize","params":{}}"#,
        Duration::from_millis(150),
    )
    .await;
    assert_eq!(code, 0);
    let reply: serde_json::Value = serde_json::from_str(out.trim()).expect("one reply line");
    assert_eq!(reply["error"]["code"], -32000, "{reply}");
    assert_eq!(reply["error"]["message"], "read timeout", "{reply}");
}

// ---------- decision 0029 §4: stdout carries protocol, or nothing ----------

/// A one-shot HTTP peer: reads one request-sized chunk, answers with a
/// canned response, drops. Enough server for a bridge whose client speaks
/// HTTP/1.1 with identity encoding.
async fn canned_server(response: &'static str) -> std::net::SocketAddr {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let mut scratch = [0u8; 8192];
        let _ = stream.read(&mut scratch).await;
        stream.write_all(response.as_bytes()).await.unwrap();
        // Drop: the bridge reads to EOF and parses what arrived.
    });
    addr
}

/// A notification stays silent when its POST *fails* (decision 0029 §4):
/// the missing-`id` predicate is applied locally in the failure arm, so a
/// line-parsing client never reads an `id: null` error object it did not
/// ask for — the stdout twin of the bare-newline bug.
#[tokio::test]
async fn a_failed_notification_stays_silent() {
    let (code, out) = bridge_over(
        "http://127.0.0.1:1",
        br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
    )
    .await;
    assert_eq!(code, 0);
    assert_eq!(out, "", "a failed notification writes nothing: {out:?}");
}

/// A notification whose POST *succeeds* but is answered with a body by a
/// misbehaving peer stays silent too: the bridge does not delegate the
/// predicate to the server.
#[tokio::test]
async fn a_misbehaving_server_body_for_a_notification_is_silent() {
    let addr = canned_server("HTTP/1.1 200 OK\r\nContent-Length: 7\r\n\r\n{\"id\":1}").await;
    let (code, out) = bridge_over(
        &format!("http://{addr}"),
        br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
    )
    .await;
    assert_eq!(code, 0);
    assert_eq!(
        out, "",
        "the body for a notification writes nothing: {out:?}"
    );
}

/// The same predication on the *forwarded* path: a well-formed 2xx JSON
/// body for an id-less line is still not protocol the client asked for —
/// the `Outcome::Json` arm checks the id it parsed itself, so even a body
/// that is a valid JSON-RPC reply writes nothing for a notification.
#[tokio::test]
async fn a_wellformed_json_body_for_an_idless_line_is_silent() {
    let addr = canned_server("HTTP/1.1 200 OK\r\nContent-Length: 7\r\n\r\n{\"x\":1}").await;
    let (code, out) = bridge_over(
        &format!("http://{addr}"),
        br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#,
    )
    .await;
    assert_eq!(code, 0);
    assert_eq!(out, "", "no id, no output line: {out:?}");
}

/// Declared silence (an empty 2xx body) writes nothing even for a request:
/// an empty line is not protocol, and a fabricated `{}` would be worse.
#[tokio::test]
async fn an_empty_body_for_a_request_writes_nothing() {
    let addr = canned_server("HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n").await;
    let (code, out) = bridge_over(
        &format!("http://{addr}"),
        br#"{"jsonrpc":"2.0","id":4,"method":"ping"}"#,
    )
    .await;
    assert_eq!(code, 0);
    assert_eq!(out, "", "an empty body is not a reply line: {out:?}");
}

/// A server-declared JSON-RPC error object is forwarded verbatim — a 400
/// parse refusal from the real server is exactly this shape.
#[tokio::test]
async fn a_server_error_object_is_forwarded_verbatim() {
    let addr = canned_server(
        "HTTP/1.1 400 Bad Request\r\nContent-Length: 48\r\n\r\n{\"jsonrpc\":\"2.0\",\"id\":5,\"error\":{\"code\":-32700}}",
    )
    .await;
    let (code, out) = bridge_over(
        &format!("http://{addr}"),
        br#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{}}"#,
    )
    .await;
    assert_eq!(code, 0);
    let reply: serde_json::Value = serde_json::from_str(out.trim()).expect("one reply line");
    assert_eq!(reply["id"], 5, "{reply}");
    assert_eq!(
        reply["error"]["code"], -32700,
        "forwarded verbatim: {reply}"
    );
}

/// A 2xx body that is not JSON refuses — it never becomes a fabricated
/// `"{}"` result (decision 0029 §4).
#[tokio::test]
async fn a_non_json_reply_refuses_instead_of_fabricating() {
    let addr = canned_server("HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello").await;
    let (code, out) = bridge_over(
        &format!("http://{addr}"),
        br#"{"jsonrpc":"2.0","id":6,"method":"ping"}"#,
    )
    .await;
    assert_eq!(code, 0);
    let reply: serde_json::Value = serde_json::from_str(out.trim()).expect("one reply line");
    assert_eq!(reply["id"], 6, "{reply}");
    assert_eq!(reply["error"]["code"], -32000, "{reply}");
    assert!(
        reply["error"]["message"]
            .as_str()
            .unwrap()
            .contains("the reply is not JSON"),
        "{reply}"
    );
}

/// The old 10-second clamp is dead: a wait longer than ten seconds, driven
/// through the real bridge against the real server, gets its result. Under
/// the deleted `READ_TIMEOUT` the bridge would have answered -32000 "read
/// timeout" at 10 s; under the derived 65 s budget the wait's own outcome
/// arrives. The result *is* the property — no wall-clock assertion.
#[tokio::test]
async fn a_wait_longer_than_ten_seconds_gets_its_result() {
    let srv = common::start().await;
    let base = format!("http://{}", srv.http_addr);
    let (code, out) = bridge_over_with_deadline(
        &base,
        br#"{"jsonrpc":"2.0","id":21,"method":"tools/call","params":{"name":"swarmail_wait_for_email","arguments":{"inbox":"slowbox","timeout_ms":15000}}}"#,
        swarmail::stdio::bridge_deadline(),
    )
    .await;
    assert_eq!(code, 0);
    let reply: serde_json::Value = serde_json::from_str(out.trim()).expect("one reply line");
    assert_eq!(reply["id"], 21, "{reply}");
    let result: serde_json::Value =
        serde_json::from_str(reply["result"]["content"][0]["text"].as_str().unwrap())
            .expect("the wait's result");
    assert_eq!(result["timed_out"], true, "the wait answered, unhurried");
}
