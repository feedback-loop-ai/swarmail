//! The stdio JSON-RPC bridge, driven in-process over duplex pipes.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
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
    let mut reader = tokio::io::BufReader::new(r);
    let code = swarmail::stdio::bridge(base_url, &mut reader, &mut bridge_output, deadline).await;
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
            pop3_idle_timeout: swarmail::pop3::DEFAULT_IDLE_TIMEOUT,
        };
        swarmail::run_on(&cfg).await.unwrap()
    }
}

#[tokio::test]
async fn short_http_bodies_fail_correlated_slots_and_the_bridge_recovers() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let peer = tokio::spawn(async move {
        for _ in 0..3 {
            let (mut stream, _) = listener.accept().await.unwrap();
            let body = read_request_body(&mut stream).await;
            let value: serde_json::Value = serde_json::from_str(&body).unwrap();
            let response = if value.is_array() {
                "HTTP/1.1 200 OK\r\nContent-Length: 99\r\n\r\n{}"
            } else if value["id"] == 8 {
                let json = r#"{"jsonrpc":"2.0","id":8,"result":{}}"#;
                return_after_write(json, &mut stream).await;
                continue;
            } else {
                "HTTP/1.1 202 Accepted\r\nContent-Length: 2\r\n\r\n "
            };
            stream.write_all(response.as_bytes()).await.unwrap();
        }
    });
    let input = concat!(
        "[{\"id\":4,\"method\":\"ping\"},",
        "{\"id\":null,\"method\":\"ping\"},",
        "{\"id\":false,\"method\":\"ping\"},",
        "{\"method\":\"notifications/initialized\"}]\n",
        "{\"method\":\"notifications/initialized\"}\n",
        "{\"id\":8,\"method\":\"ping\"}\n"
    );
    let (code, out) = bridge_over(&format!("http://{addr}"), input.as_bytes()).await;
    assert_eq!(code, 0);
    peer.await.unwrap();
    let replies = reply_lines(&out);
    assert_eq!(
        replies.len(),
        2,
        "the truncated notification is silent: {out:?}"
    );
    let batch = replies[0].as_array().expect("one batch error array");
    assert_eq!(
        batch
            .iter()
            .map(|reply| reply["id"].clone())
            .collect::<Vec<_>>(),
        [
            serde_json::json!(4),
            serde_json::Value::Null,
            serde_json::Value::Null
        ]
    );
    assert!(
        batch.iter().all(|reply| reply["error"]["code"] == -32000),
        "{batch:?}"
    );
    assert_eq!(replies[1]["id"], 8, "{out:?}");
    assert_eq!(replies[1]["result"], serde_json::json!({}), "{out:?}");
}

async fn return_after_write(json: &str, stream: &mut tokio::net::TcpStream) {
    stream
        .write_all(
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{json}",
                json.len()
            )
            .as_bytes(),
        )
        .await
        .unwrap();
}

#[tokio::test]
async fn invalid_id_refusal_and_valid_notification_effects_match_over_stdio() {
    let srv = common::start().await;
    let base = format!("http://{}", srv.http_addr);
    let (code, refused) = bridge_over(
        &base,
        br#"{"id":true,"method":"tools/call","params":{"name":"swarmail_seed_email","arguments":{"inbox":"bad-id","to":"a@x.io"}}}"#,
    )
    .await;
    assert_eq!(code, 0);
    let reply = reply_lines(&refused);
    assert_eq!(reply.len(), 1, "{refused:?}");
    assert_eq!(reply[0]["error"]["code"], -32600, "{refused:?}");
    assert_eq!(reply[0]["id"], serde_json::Value::Null, "{refused:?}");

    let (code, notification) = bridge_over(
        &base,
        br#"{"method":"tools/call","params":{"name":"swarmail_seed_email","arguments":{"inbox":"good-id","to":"a@x.io"}}}"#,
    )
    .await;
    assert_eq!(code, 0);
    assert!(
        notification.is_empty(),
        "valid notification is silent: {notification:?}"
    );

    for (inbox, count) in [("bad-id", 0), ("good-id", 1)] {
        let request = format!(
            r#"{{"id":"read-{inbox}","method":"tools/call","params":{{"name":"swarmail_search_emails","arguments":{{"inbox":"{inbox}"}}}}}}"#
        );
        let (code, output) = bridge_over(&base, request.as_bytes()).await;
        assert_eq!(code, 0);
        let replies = reply_lines(&output);
        let data: serde_json::Value =
            serde_json::from_str(replies[0]["result"]["content"][0]["text"].as_str().unwrap())
                .unwrap();
        assert_eq!(data["total"], count, "{inbox}: {data}");
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
    let mut reader = tokio::io::BufReader::new(Boom);
    let mut sink = duplex(1024).0;
    let code = swarmail::stdio::bridge(
        "http://127.0.0.1:1",
        &mut reader,
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
    let (seen_tx, seen_rx) = tokio::sync::oneshot::channel::<String>();
    let (release, held) = tokio::sync::oneshot::channel::<()>();
    let mut peer = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        let request = read_request_body(&mut stream).await;
        seen_tx.send(request).unwrap();
        let _ = held.await;
        drop(stream);
    });
    let base = format!("http://{addr}");
    let (seen, output) = tokio::join!(
        tokio::time::timeout(Duration::from_secs(30), seen_rx),
        tokio::time::timeout(
            Duration::from_secs(30),
            bridge_over_with_deadline(
                &base,
                br#"{"jsonrpc":"2.0","id":11,"method":"initialize","params":{}}"#,
                Duration::from_millis(150),
            )
        )
    );
    // Release and join owned work even when an event/outcome failed.
    drop(release);
    let joined = tokio::time::timeout(Duration::from_secs(30), &mut peer).await;
    if joined.is_err() {
        peer.abort();
        let _ = peer.await;
    }
    joined
        .expect("silent peer did not finish after release")
        .expect("silent peer failed");
    let request: serde_json::Value = serde_json::from_str(
        &seen
            .expect("silent peer never observed the request")
            .expect("peer lost request event"),
    )
    .unwrap();
    assert_eq!(request["id"], 11);
    let (code, out) = output.expect("bridge did not report its read-timeout outcome");
    assert_eq!(code, 0);
    let reply: serde_json::Value = serde_json::from_str(out.trim()).expect("one reply line");
    assert_eq!(reply["id"], 11, "{reply}");
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

// ---------- decision 0039: batches, bad lines, concurrency, lifecycle ----------

/// Every stdout line parsed on its own — a blank or split line fails here.
fn reply_lines(out: &str) -> Vec<serde_json::Value> {
    out.lines()
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("{l:?} is not JSON: {e}")))
        .collect()
}

/// A request batch is forwarded as the server's own array, on one line
/// (issue #72): the old bridge read an array as a notification and wrote
/// nothing at all.
#[tokio::test]
async fn a_request_batch_is_forwarded_as_one_array_line() {
    let srv = common::start().await;
    let base = format!("http://{}", srv.http_addr);
    let (code, out) = bridge_over(
        &base,
        br#"[{"jsonrpc":"2.0","id":1,"method":"ping"},{"jsonrpc":"2.0","method":"notifications/initialized"},{"jsonrpc":"2.0","id":2,"method":"nope"}]
"#,
    )
    .await;
    assert_eq!(code, 0);
    let replies = reply_lines(&out);
    assert_eq!(replies.len(), 1, "one batch, one line: {out:?}");
    let answers = replies[0].as_array().expect("an array reply");
    assert_eq!(answers.len(), 2, "the notification is unanswered: {out}");
    assert_eq!(answers[0]["id"], 1, "{out}");
    assert_eq!(answers[0]["result"], serde_json::json!({}), "{out}");
    assert_eq!(answers[1]["id"], 2, "{out}");
    assert_eq!(answers[1]["error"]["code"], -32601, "{out}");
}

/// Envelope refusals are forwarded, and notification batches stay silent
/// (decision 0039 §2, updated by 0044 §1): `[]` and a 17-member
/// array — even one of notifications only — get the server's single
/// -32600, and so does a scalar line, which is an invalid request, not a
/// notification; an all-notification batch writes nothing. Red before:
/// the scalar line was silent.
#[tokio::test]
async fn batch_refusals_are_forwarded_and_notification_batches_stay_silent() {
    let srv = common::start().await;
    let base = format!("http://{}", srv.http_addr);
    let note = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    let input = format!("[]\n[{}]\n[{note},{note}]\n42\n", vec![note; 17].join(","));
    let (code, out) = bridge_over(&base, input.as_bytes()).await;
    assert_eq!(code, 0);
    let replies = reply_lines(&out);
    assert_eq!(replies.len(), 3, "three refusals, nothing else: {out:?}");
    for reply in &replies {
        assert_eq!(reply["id"], serde_json::Value::Null, "{reply}");
        assert_eq!(reply["error"]["code"], -32600, "{reply}");
    }
}

/// Invalid batch members are answered through the bridge in both arms
/// (decision 0044 §1): against the real server each non-object member gets
/// its own null-id -32600 in member order beside the request's reply; when
/// the exchange fails, the transport error carries a null-id position for
/// each, and a scalar line gets one null-id error instead of silence.
#[tokio::test]
async fn invalid_batch_members_are_answered_in_both_arms() {
    let ping = r#"{"jsonrpc":"2.0","id":1,"method":"ping"}"#;
    let note = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    let batch = format!("[{ping},5,{note},[]]");

    let srv = common::start().await;
    let base = format!("http://{}", srv.http_addr);
    let (code, out) = bridge_over(&base, format!("{batch}\n").as_bytes()).await;
    assert_eq!(code, 0);
    let replies = reply_lines(&out);
    assert_eq!(replies.len(), 1, "{out:?}");
    let answers = replies[0].as_array().expect("an array reply");
    let ids: Vec<&serde_json::Value> = answers.iter().map(|e| &e["id"]).collect();
    let null = serde_json::Value::Null;
    assert_eq!(ids, [&serde_json::json!(1), &null, &null], "{out}");
    assert_eq!(answers[1]["error"]["code"], -32600, "{out}");
    assert_eq!(answers[2]["error"]["code"], -32600, "{out}");

    let (code, out) = bridge_over("http://127.0.0.1:1", format!("{batch}\n7\n").as_bytes()).await;
    assert_eq!(code, 0);
    let replies = reply_lines(&out);
    assert_eq!(replies.len(), 2, "{out:?}");
    let failed = replies.iter().find(|r| r.is_array()).expect("the batch");
    let ids: Vec<&serde_json::Value> = failed
        .as_array()
        .unwrap()
        .iter()
        .map(|e| &e["id"])
        .collect();
    assert_eq!(ids, [&serde_json::json!(1), &null, &null], "{failed}");
    let scalar = replies.iter().find(|r| r.is_object()).expect("the scalar");
    assert_eq!(scalar["id"], null, "{scalar}");
    assert_eq!(scalar["error"]["code"], -32000, "{scalar}");
}

/// A peer body for an all-notification batch is not a reply the client
/// asked for: silence holds whatever the peer sends.
#[tokio::test]
async fn a_spurious_body_for_a_notification_batch_is_silent() {
    let addr = canned_server("HTTP/1.1 200 OK\r\nContent-Length: 9\r\n\r\n[{\"x\":1}]").await;
    let (code, out) = bridge_over(
        &format!("http://{addr}"),
        br#"[{"jsonrpc":"2.0","method":"notifications/initialized"}]"#,
    )
    .await;
    assert_eq!(code, 0);
    assert_eq!(out, "", "{out:?}");
}

/// A failed exchange answers every id its line carried (decision 0039 §3):
/// an admitted batch gets one array — duplicates, null and order kept,
/// notifications excluded; an empty or over-cap batch gets one null-id
/// error whatever ids it holds; a notification batch stays silent.
#[tokio::test]
async fn failed_batches_answer_by_the_correlation_table() {
    let ping = |id: &str| format!(r#"{{"jsonrpc":"2.0","id":{id},"method":"ping"}}"#);
    let note = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    let admitted = format!("[{},{note},{},{}]", ping("1"), ping("null"), ping("1"));
    let over_cap = format!("[{}]", vec![ping("5"); 17].join(","));
    let input = format!("{admitted}\n[{note}]\n{over_cap}\n");
    let (code, out) = bridge_over("http://127.0.0.1:1", input.as_bytes()).await;
    assert_eq!(code, 0);
    let replies = reply_lines(&out);
    assert_eq!(replies.len(), 2, "{out:?}");
    let batch = replies
        .iter()
        .find(|r| r.is_array())
        .expect("the admitted batch's array");
    let ids: Vec<&serde_json::Value> = batch.as_array().unwrap().iter().map(|e| &e["id"]).collect();
    assert_eq!(
        ids,
        [
            &serde_json::json!(1),
            &serde_json::Value::Null,
            &serde_json::json!(1)
        ],
        "{batch}"
    );
    for e in batch.as_array().unwrap() {
        assert_eq!(e["error"]["code"], -32000, "{batch}");
    }
    let refused = replies
        .iter()
        .find(|r| r.is_object())
        .expect("the over-cap batch's one error");
    assert_eq!(refused["id"], serde_json::Value::Null, "{refused}");
    assert_eq!(refused["error"]["code"], -32000, "{refused}");
}

/// A JSON body that spans lines leaves the bridge as one compact line: the
/// value is forwarded, never its whitespace.
#[tokio::test]
async fn a_multiline_reply_is_written_as_one_line() {
    let addr = canned_server("HTTP/1.1 200 OK\r\n\r\n{\n  \"id\": 8,\n  \"result\": {}\n}\n").await;
    let (code, out) = bridge_over(
        &format!("http://{addr}"),
        br#"{"jsonrpc":"2.0","id":8,"method":"ping"}"#,
    )
    .await;
    assert_eq!(code, 0);
    assert_eq!(out, "{\"id\":8,\"result\":{}}\n");
}

/// An HTTP peer that answers every exchange with the same canned reply and
/// counts the exchanges it saw.
async fn counting_server(response: &'static str) -> (std::net::SocketAddr, Arc<AtomicUsize>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let seen = Arc::new(AtomicUsize::new(0));
    let counter = seen.clone();
    tokio::spawn(async move {
        loop {
            let (mut stream, _) = listener.accept().await.unwrap();
            counter.fetch_add(1, Ordering::SeqCst);
            let _ = read_request_body(&mut stream).await;
            stream.write_all(response.as_bytes()).await.unwrap();
        }
    });
    (addr, seen)
}

/// A bad line is answered locally and the bridge keeps reading (issue
/// #90): a non-UTF-8 line and a malformed one each get one null-id -32700,
/// neither reaches the network, and the ping behind them is answered.
#[tokio::test]
async fn bad_lines_are_answered_locally_and_the_next_request_still_runs() {
    let (addr, seen) = counting_server(
        "HTTP/1.1 200 OK\r\nContent-Length: 36\r\n\r\n{\"jsonrpc\":\"2.0\",\"id\":3,\"result\":{}}",
    )
    .await;
    let (code, out) = bridge_over(
        &format!("http://{addr}"),
        b"\xff\xfe\n{nope\n{\"jsonrpc\":\"2.0\",\"id\":3,\"method\":\"ping\"}\n",
    )
    .await;
    assert_eq!(code, 0);
    let replies = reply_lines(&out);
    assert_eq!(replies.len(), 3, "{out:?}");
    let parse_errors: Vec<_> = replies
        .iter()
        .filter(|r| r["error"]["code"] == -32700)
        .collect();
    assert_eq!(parse_errors.len(), 2, "{out}");
    for e in parse_errors {
        assert_eq!(e["id"], serde_json::Value::Null, "{e}");
    }
    assert!(replies.iter().any(|r| r["id"] == 3), "{out}");
    assert_eq!(
        seen.load(Ordering::SeqCst),
        1,
        "only the ping reached the network"
    );
}

/// CRLF endings, whitespace-only lines and a final line without a newline
/// all frame correctly: two requests in, two replies out.
#[tokio::test]
async fn crlf_blank_and_unterminated_lines_frame_correctly() {
    let srv = common::start().await;
    let base = format!("http://{}", srv.http_addr);
    let (code, out) = bridge_over(
        &base,
        b"\r\n   \r\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\r\n{\"jsonrpc\":\"2.0\",\"id\":2,\"method\":\"ping\"}",
    )
    .await;
    assert_eq!(code, 0);
    let mut ids: Vec<i64> = reply_lines(&out)
        .iter()
        .map(|r| r["id"].as_i64().unwrap())
        .collect();
    ids.sort();
    assert_eq!(ids, [1, 2], "{out:?}");
}

/// Read one HTTP request off the wire and return its body.
async fn read_request_body(stream: &mut tokio::net::TcpStream) -> String {
    read_request_with_head(stream).await.1
}

async fn read_request_with_head(stream: &mut tokio::net::TcpStream) -> (String, String) {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        if let Some(sep) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            let head = String::from_utf8_lossy(&buf[..sep]).to_string();
            let len: usize = head
                .lines()
                .find_map(|l| l.strip_prefix("Content-Length: "))
                .expect("the bridge declares its length")
                .parse()
                .unwrap();
            if buf.len() >= sep + 4 + len {
                return (
                    head,
                    String::from_utf8(buf[sep + 4..sep + 4 + len].to_vec()).unwrap(),
                );
            }
        }
        let n = stream.read(&mut chunk).await.unwrap();
        assert!(n > 0, "the bridge closed mid-request");
        buf.extend_from_slice(&chunk[..n]);
    }
}

/// An HTTP peer that holds every exchange until the test answers it: each
/// accepted connection arrives with its request body.
struct HeldPeer {
    addr: std::net::SocketAddr,
    headers: Arc<Mutex<Vec<String>>>,
    exchanges: tokio::sync::mpsc::UnboundedReceiver<(String, tokio::net::TcpStream)>,
}

impl HeldPeer {
    async fn start() -> HeldPeer {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, exchanges) = tokio::sync::mpsc::unbounded_channel();
        let headers = Arc::new(Mutex::new(Vec::new()));
        let seen_headers = headers.clone();
        tokio::spawn(async move {
            loop {
                let (mut stream, _) = listener.accept().await.unwrap();
                let tx = tx.clone();
                let headers = seen_headers.clone();
                tokio::spawn(async move {
                    let (head, body) = read_request_with_head(&mut stream).await;
                    headers.lock().unwrap().push(head);
                    let _ = tx.send((body, stream));
                });
            }
        });
        HeldPeer {
            addr,
            exchanges,
            headers,
        }
    }

    /// The next held exchange, bounded so a bridge that never sends fails.
    async fn held(&mut self) -> (String, tokio::net::TcpStream) {
        tokio::time::timeout(Duration::from_secs(30), self.exchanges.recv())
            .await
            .expect("the bridge never sent the exchange")
            .expect("the peer is alive")
    }
}

/// Release a held exchange: a request gets a `ping` result carrying its own
/// id; a notification gets the server's empty 202.
async fn release(body: &str, mut stream: tokio::net::TcpStream) {
    let req: serde_json::Value = serde_json::from_str(body).unwrap();
    let reply = match req.get("id") {
        Some(id) => {
            let json = serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": {} }).to_string();
            format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n{json}",
                json.len()
            )
        }
        None => "HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\n\r\n".to_string(),
    };
    stream.write_all(reply.as_bytes()).await.unwrap();
}

/// A reader that publishes how many bytes the bridge has consumed: the
/// observation point for admission, never a sleep.
struct Consumed<R> {
    inner: R,
    consumed: tokio::sync::watch::Sender<usize>,
}

impl<R: tokio::io::AsyncRead + Unpin> tokio::io::AsyncRead for Consumed<R> {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::pin::Pin::new(&mut self.inner).poll_read(cx, buf)
    }
}

impl<R: tokio::io::AsyncBufRead + Unpin> tokio::io::AsyncBufRead for Consumed<R> {
    fn poll_fill_buf(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<&[u8]>> {
        std::pin::Pin::new(&mut self.get_mut().inner).poll_fill_buf(cx)
    }

    fn consume(mut self: std::pin::Pin<&mut Self>, amt: usize) {
        self.consumed.send_modify(|n| *n += amt);
        std::pin::Pin::new(&mut self.inner).consume(amt);
    }
}

/// A bridge running in its own task, fed and read by the test as it goes.
struct Running {
    input: tokio::io::DuplexStream,
    output: tokio::io::BufReader<tokio::io::DuplexStream>,
    consumed: tokio::sync::watch::Receiver<usize>,
    task: tokio::task::JoinHandle<i32>,
}

impl Running {
    fn spawn(base_url: String) -> Running {
        let (input, bridge_input) = duplex(64 * 1024);
        let (mut bridge_output, output) = duplex(64 * 1024);
        let (tx, consumed) = tokio::sync::watch::channel(0);
        let task = tokio::spawn(async move {
            let mut reader = Consumed {
                inner: tokio::io::BufReader::new(bridge_input),
                consumed: tx,
            };
            swarmail::stdio::bridge(
                &base_url,
                &mut reader,
                &mut bridge_output,
                Duration::from_secs(3600),
            )
            .await
        });
        Running {
            input,
            output: tokio::io::BufReader::new(output),
            consumed,
            task,
        }
    }

    async fn send(&mut self, bytes: &str) {
        self.input.write_all(bytes.as_bytes()).await.unwrap();
    }

    /// Wait (bounded) until the bridge has consumed exactly `n` input bytes.
    async fn consumed_reaches(&mut self, n: usize) {
        tokio::time::timeout(Duration::from_secs(30), self.consumed.wait_for(|c| *c == n))
            .await
            .expect("the bridge never consumed the bytes")
            .expect("the bridge is alive");
    }

    async fn reply(&mut self) -> serde_json::Value {
        let mut line = String::new();
        tokio::time::timeout(Duration::from_secs(30), self.output.read_line(&mut line))
            .await
            .expect("no reply line")
            .unwrap();
        serde_json::from_str(&line).unwrap_or_else(|e| panic!("{line:?}: {e}"))
    }

    /// Close stdin, then collect every remaining reply and the exit code.
    async fn finish(self) -> (i32, Vec<serde_json::Value>) {
        let Running {
            input,
            mut output,
            task,
            ..
        } = self;
        drop(input);
        let code = tokio::time::timeout(Duration::from_secs(30), task)
            .await
            .expect("the bridge never exited")
            .unwrap();
        let mut rest = String::new();
        output.read_to_string(&mut rest).await.unwrap();
        (code, reply_lines(&rest))
    }
}

fn ping_line(id: &str) -> String {
    format!("{{\"jsonrpc\":\"2.0\",\"id\":{id},\"method\":\"ping\"}}\n")
}

/// Admission is bounded (decision 0039 §4): sixteen held exchanges — a
/// notification among them — fill the bridge, and the seventeenth line
/// stays unread until one of them completes.
#[tokio::test]
async fn at_capacity_the_next_line_waits_until_an_exchange_completes() {
    let mut peer = HeldPeer::start().await;
    let mut bridge = Running::spawn(format!("http://{}", peer.addr));
    let note = "{\"jsonrpc\":\"2.0\",\"method\":\"notifications/initialized\"}\n";
    let mut admitted: String = (0..15).map(|i| ping_line(&i.to_string())).collect();
    admitted.push_str(note);
    let late = ping_line("\"late\"");
    bridge.send(&format!("{admitted}{late}")).await;

    let mut held = Vec::new();
    for _ in 0..swarmail::stdio::BRIDGE_CONCURRENCY {
        held.push(peer.held().await);
    }
    // Sixteen exchanges outstanding and none complete: the reader stopped
    // exactly at the admitted lines.
    assert_eq!(*bridge.consumed.borrow(), admitted.len());

    // The notification completing frees a slot: the late line is read.
    let at = held
        .iter()
        .position(|(body, _)| !body.contains("\"id\""))
        .expect("the notification is among the held");
    let (body, stream) = held.remove(at);
    release(&body, stream).await;
    bridge.consumed_reaches(admitted.len() + late.len()).await;
    held.push(peer.held().await);

    for (body, stream) in held {
        release(&body, stream).await;
    }
    let (code, replies) = bridge.finish().await;
    assert_eq!(code, 0);
    assert_eq!(replies.len(), 16, "fifteen pings and the late one");
    assert!(replies.iter().any(|r| r["id"] == "late"));
}

/// A completion that lands while a line is half-read neither loses nor
/// duplicates it: the prefix survives in the bridge's buffer.
#[tokio::test]
async fn a_completion_mid_line_keeps_the_partial_line() {
    let mut peer = HeldPeer::start().await;
    let mut bridge = Running::spawn(format!("http://{}", peer.addr));
    let first = ping_line("1");
    bridge.send(&first).await;
    let (body, stream) = peer.held().await;

    let second = ping_line("2");
    let (prefix, suffix) = second.split_at(12);
    bridge.send(prefix).await;
    bridge.consumed_reaches(first.len() + prefix.len()).await;
    release(&body, stream).await;
    assert_eq!(bridge.reply().await["id"], 1);

    bridge.send(suffix).await;
    let (body, stream) = peer.held().await;
    assert_eq!(body, second.trim(), "the line arrives whole, once");
    release(&body, stream).await;
    let (code, replies) = bridge.finish().await;
    assert_eq!(code, 0);
    assert_eq!(replies.len(), 1, "{replies:?}");
    assert_eq!(replies[0]["id"], 2);
}

/// EOF with work outstanding drains it (issue #90): every admitted reply is
/// written before the bridge exits 0.
#[tokio::test]
async fn eof_drains_every_admitted_exchange() {
    let mut peer = HeldPeer::start().await;
    let mut bridge = Running::spawn(format!("http://{}", peer.addr));
    bridge
        .send(&format!(
            "{}{}{}",
            ping_line("1"),
            ping_line("2"),
            ping_line("3")
        ))
        .await;
    let held = [peer.held().await, peer.held().await, peer.held().await];
    let Running {
        input,
        mut output,
        task,
        ..
    } = bridge;
    drop(input);
    assert!(!task.is_finished(), "three exchanges are still held");
    for (body, stream) in held {
        release(&body, stream).await;
    }
    let code = tokio::time::timeout(Duration::from_secs(30), task)
        .await
        .expect("the bridge never exited")
        .unwrap();
    assert_eq!(code, 0);
    let mut rest = String::new();
    output.read_to_string(&mut rest).await.unwrap();
    assert_eq!(reply_lines(&rest).len(), 3, "{rest:?}");
}

/// A reader that hands over one chunk and then fails.
struct ThenFail(Option<&'static [u8]>);

impl tokio::io::AsyncRead for ThenFail {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(match self.0.take() {
            Some(chunk) => {
                buf.put_slice(chunk);
                Ok(())
            }
            None => Err(std::io::Error::other("boom")),
        })
    }
}

/// A stdin failure stops reading but not answering: the request admitted
/// before it still gets its reply, then the bridge exits 1.
#[tokio::test]
async fn a_stdin_error_drains_admitted_replies_then_exits_one() {
    let srv = common::start().await;
    let base = format!("http://{}", srv.http_addr);
    let mut reader = tokio::io::BufReader::new(ThenFail(Some(
        b"{\"jsonrpc\":\"2.0\",\"id\":4,\"method\":\"ping\"}\n{\"partial",
    )));
    let mut out = Vec::new();
    let code = swarmail::stdio::bridge(&base, &mut reader, &mut out, Duration::from_secs(10)).await;
    assert_eq!(code, 1);
    let out = String::from_utf8(out).unwrap();
    let replies = reply_lines(&out);
    assert_eq!(replies.len(), 1, "the fragment is not a request: {out:?}");
    assert_eq!(replies[0]["id"], 4);
}

/// A stdout that records each offered write and flush, or refuses one.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Fault {
    None,
    Write,
    Flush,
}

struct Stdout {
    fault: Fault,
    log: Arc<Mutex<Vec<String>>>,
}

impl tokio::io::AsyncWrite for Stdout {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        if self.fault == Fault::Write {
            return std::task::Poll::Ready(Err(std::io::Error::other("stdout closed")));
        }
        let mut log = self.log.lock().unwrap();
        log.push(format!("write {}", String::from_utf8_lossy(buf)));
        std::task::Poll::Ready(Ok(buf.len()))
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        if self.fault == Fault::Flush {
            return std::task::Poll::Ready(Err(std::io::Error::other("stdout closed")));
        }
        self.log.lock().unwrap().push("flush".to_string());
        std::task::Poll::Ready(Ok(()))
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        std::task::Poll::Ready(Ok(()))
    }
}

/// Each reply — local or forwarded — is offered whole, newline included, to
/// one write, and flushed after it (decision 0039 §5).
#[tokio::test]
async fn every_reply_is_one_complete_write_then_a_flush() {
    let srv = common::start().await;
    let base = format!("http://{}", srv.http_addr);
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut stdout = Stdout {
        fault: Fault::None,
        log: log.clone(),
    };
    let mut reader = tokio::io::BufReader::new(
        &b"{nope\n{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n"[..],
    );
    let code =
        swarmail::stdio::bridge(&base, &mut reader, &mut stdout, Duration::from_secs(10)).await;
    assert_eq!(code, 0);
    let log = log.lock().unwrap().clone();
    assert_eq!(log.len(), 4, "{log:?}");
    for pair in log.chunks(2) {
        let line = pair[0].strip_prefix("write ").expect("a write first");
        assert!(line.ends_with('\n'), "{line:?}");
        assert_eq!(line.matches('\n').count(), 1, "{line:?}");
        let _: serde_json::Value = serde_json::from_str(line).expect("a complete JSON line");
        assert_eq!(pair[1], "flush", "{log:?}");
    }
}

/// A stdout failure ends the bridge at once with 1 — it does not wait out
/// the long exchange still held at the peer (decision 0039 §5). Both the
/// write and the flush failure, on the local-reply path.
#[tokio::test]
async fn a_stdout_failure_exits_one_without_waiting_for_held_work() {
    for fault in [Fault::Write, Fault::Flush] {
        let mut peer = HeldPeer::start().await;
        let base = format!("http://{}", peer.addr);
        let (mut input, bridge_input) = duplex(64 * 1024);
        let task = tokio::spawn(async move {
            let mut reader = tokio::io::BufReader::new(bridge_input);
            let mut stdout = Stdout {
                fault,
                log: Arc::new(Mutex::new(Vec::new())),
            };
            swarmail::stdio::bridge(&base, &mut reader, &mut stdout, Duration::from_secs(3600))
                .await
        });
        input.write_all(ping_line("1").as_bytes()).await.unwrap();
        let _held = peer.held().await;
        input.write_all(b"{nope\n").await.unwrap();
        let code = tokio::time::timeout(Duration::from_secs(30), task)
            .await
            .expect("the bridge waited on held work")
            .unwrap();
        assert_eq!(code, 1);
    }
}

/// The same failure on the completion path: a reply that cannot be written
/// ends the bridge with 1.
#[tokio::test]
async fn a_reply_that_cannot_be_written_exits_one() {
    let mut stdout = Stdout {
        fault: Fault::Write,
        log: Arc::new(Mutex::new(Vec::new())),
    };
    let mut reader =
        tokio::io::BufReader::new(&b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}\n"[..]);
    let code = swarmail::stdio::bridge(
        "http://127.0.0.1:1",
        &mut reader,
        &mut stdout,
        Duration::from_secs(10),
    )
    .await;
    assert_eq!(code, 1);
}

#[path = "common/mod.rs"]
mod report_wire;

#[tokio::test]
async fn report_stdio_text_contract() {
    let srv = common::start().await;
    for raw in [
        &b"\r\nempty\r\n"[..],
        b"X-Swarmail-Demo: warning\r\n\r\nbody\r\n",
    ] {
        let id = report_wire::smtp_raw(srv.smtp_addr, "stdio-report", raw).await;
        let expected = report_wire::http_json(
            srv.http_addr,
            "GET",
            &format!("/api/v1/messages/{id}/deliverability"),
            None,
        )
        .await
        .1;
        let input = serde_json::json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
            "name":"swarmail_get_deliverability_report","arguments":{"id":id}}});
        let (code, output) = bridge_over(
            &format!("http://{}", srv.http_addr),
            format!("{input}\n").as_bytes(),
        )
        .await;
        assert_eq!(code, 0);
        let reply: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
        let result = &reply["result"];
        assert_eq!(result["isError"], false);
        assert!(result.get("structuredContent").is_none());
        assert_eq!(result["content"].as_array().unwrap().len(), 1);
        assert_eq!(result["content"][0]["type"], "text");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(
                result["content"][0]["text"].as_str().unwrap()
            )
            .unwrap(),
            expected
        );
    }
    for (arguments, code) in [
        (serde_json::json!({}), -32602),
        (serde_json::json!({"id":null}), -32602),
    ] {
        let input = serde_json::json!({"id":1,"method":"tools/call","params":{"name":"swarmail_get_deliverability_report","arguments":arguments}});
        let (_, output) = bridge_over(
            &format!("http://{}", srv.http_addr),
            format!("{input}\n").as_bytes(),
        )
        .await;
        let reply: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
        assert_eq!(reply["error"]["code"], code);
    }
    let input = b"{\"id\":1,\"method\":\"tools/call\",\"params\":{\"name\":\"swarmail_get_deliverability_report\",\"arguments\":{\"id\":\"missing\"}}}\n";
    let (_, output) = bridge_over(&format!("http://{}", srv.http_addr), input).await;
    let reply: serde_json::Value = serde_json::from_str(output.trim()).unwrap();
    assert_eq!(reply["result"]["isError"], true);
    srv.stop().await;
}

fn revision_init(id: i64, version: &str) -> String {
    format!(
        "{}\n",
        serde_json::json!({"jsonrpc":"2.0","id":id,"method":"initialize","params":{"protocolVersion":version}})
    )
}

async fn revision_result(stream: &mut tokio::net::TcpStream, id: i64, version: &str) {
    return_after_write(
        &serde_json::json!({"jsonrpc":"2.0","id":id,"result":{"protocolVersion":version,"capabilities":{"tools":{}},"serverInfo":{"name":"peer","version":"1"}}})
            .to_string(),
        stream,
    )
    .await;
}

/// The input-consumption observer proves fences causally, even with an earlier
/// exchange still held. Returned revision, rather than proposed revision, wins.
#[tokio::test]
async fn revision_exchange_fences_and_headers_follow_the_result() {
    let mut peer = HeldPeer::start().await;
    let mut bridge = Running::spawn(format!("http://{}", peer.addr));
    bridge.send(&ping_line("0")).await;
    let (earlier, earlier_stream) = peer.held().await;
    let init = revision_init(1, "2025-06-18");
    let late = ping_line("2");
    bridge.send(&format!("{init}{late}")).await;
    let (body, mut stream) = peer.held().await;
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&body).unwrap()["id"],
        1
    );
    assert_eq!(*bridge.consumed.borrow(), ping_line("0").len() + init.len());
    assert!(peer.headers.lock().unwrap().iter().all(|h| {
        h.contains("Accept: application/json, text/event-stream")
            && h.contains("Content-Type: application/json")
            && !h.contains("MCP-Protocol-Version")
    }));
    revision_result(&mut stream, 1, "2025-11-25").await;
    drop(stream);
    assert_eq!(
        bridge.reply().await["result"]["protocolVersion"],
        "2025-11-25"
    );
    let (body, stream) = peer.held().await;
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&body).unwrap()["id"],
        2
    );
    assert!(
        peer.headers
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .contains("MCP-Protocol-Version: 2025-11-25\r\n")
    );
    release(&body, stream).await;
    assert_eq!(bridge.reply().await["id"], 2);
    release(&earlier, earlier_stream).await;
    assert_eq!(bridge.reply().await["id"], 0);
    let note = "{\"method\":\"notifications/initialized\"}\n";
    let operation = "{\"id\":3,\"method\":\"tools/list\"}\n";
    bridge.send(&format!("{note}{operation}")).await;
    let (_, mut stream) = peer.held().await;
    assert_eq!(
        *bridge.consumed.borrow(),
        ping_line("0").len() + init.len() + late.len() + note.len()
    );
    stream
        .write_all(b"HTTP/1.1 202 Accepted\r\nContent-Length: 0\r\n\r\n")
        .await
        .unwrap();
    drop(stream);
    let (body, stream) = peer.held().await;
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&body).unwrap()["id"],
        3
    );
    release(&body, stream).await;
    assert_eq!(bridge.reply().await["id"], 3);
    let (code, replies) = bridge.finish().await;
    assert_eq!(code, 0);
    assert!(replies.is_empty());
}

#[tokio::test]
async fn failed_modern_negotiation_never_releases_queued_effects_as_legacy() {
    use serde_json::json;
    for (status, result) in [
        (
            200,
            json!({"jsonrpc":"2.0","id":99,"result":{"protocolVersion":"2025-06-18"}}),
        ),
        (
            200,
            json!({"jsonrpc":"2.0","id":1,"result":{"protocolVersion":"future"}}),
        ),
        (200, json!({"jsonrpc":"2.0","id":1,"result":{}})),
        (
            200,
            json!({"jsonrpc":"2.0","id":1,"result":{"protocolVersion":null}}),
        ),
        (
            200,
            json!({"id":1,"result":{"protocolVersion":"2025-06-18"}}),
        ),
        (200, json!({"jsonrpc":"2.0","id":1,"error":{"code":-32600}})),
        (
            400,
            json!({"jsonrpc":"2.0","id":1,"error":{"code":-32600},"result":{"protocolVersion":"2025-06-18"}}),
        ),
    ] {
        let mut peer = HeldPeer::start().await;
        let mut bridge = Running::spawn(format!("http://{}", peer.addr));
        bridge
            .send(&format!(
                "{}{{\"id\":2,\"method\":\"tools/list\"}}\n{{\"method\":\"tools/call\"}}\n{}",
                revision_init(1, "2025-06-18"),
                ping_line("3")
            ))
            .await;
        let (_, mut stream) = peer.held().await;
        let data = result.to_string();
        stream
            .write_all(
                format!(
                    "HTTP/1.1 {status} Reply\r\nContent-Length: {}\r\n\r\n{data}",
                    data.len()
                )
                .as_bytes(),
            )
            .await
            .unwrap();
        drop(stream);
        assert_eq!(
            bridge.reply().await,
            result,
            "upstream replies are forwarded verbatim"
        );
        assert_eq!(bridge.reply().await["error"]["code"], -32600);
        let (body, stream) = peer.held().await;
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&body).unwrap()["id"],
            3,
            "only ping was posted"
        );
        assert!(
            !peer
                .headers
                .lock()
                .unwrap()
                .last()
                .unwrap()
                .contains("MCP-Protocol-Version"),
            "failed initialize cannot install a revision"
        );
        release(&body, stream).await;
        assert_eq!(bridge.reply().await["id"], 3);
        assert!(peer.exchanges.try_recv().is_err());
        // Explicit retry can recover into legacy and ordinary calls resume.
        bridge.send(&revision_init(4, "2025-03-26")).await;
        let (_, mut stream) = peer.held().await;
        revision_result(&mut stream, 4, "2025-03-26").await;
        drop(stream);
        assert_eq!(
            bridge.reply().await["result"]["protocolVersion"],
            "2025-03-26"
        );
        bridge.send("{\"id\":5,\"method\":\"tools/list\"}\n").await;
        let (body, stream) = peer.held().await;
        assert!(
            peer.headers
                .lock()
                .unwrap()
                .last()
                .unwrap()
                .contains("MCP-Protocol-Version: 2025-03-26")
        );
        release(&body, stream).await;
        assert_eq!(bridge.reply().await["id"], 5);
        let (code, rest) = bridge.finish().await;
        assert_eq!(code, 0);
        assert!(rest.is_empty());
    }
}

#[tokio::test]
async fn initialized_failure_stays_fenced_and_modern_arrays_never_dial() {
    let mut peer = HeldPeer::start().await;
    let mut bridge = Running::spawn(format!("http://{}", peer.addr));
    bridge.send(&revision_init(1, "2025-11-25")).await;
    let (_, mut stream) = peer.held().await;
    revision_result(&mut stream, 1, "2025-11-25").await;
    drop(stream);
    assert_eq!(bridge.reply().await["id"], 1);
    for response in [
        "HTTP/1.1 200 OK\r\nContent-Length: 0\r\n\r\n",
        "HTTP/1.1 202 Accepted\r\nContent-Length: 1\r\n\r\n ",
        "HTTP/1.1 202 Accepted\r\nContent-Length: 2\r\n\r\n{}",
        "HTTP/1.1 500 Error\r\nContent-Length: 0\r\n\r\n",
    ] {
        bridge.send("{\"method\":\"notifications/initialized\"}\n{\"id\":2,\"method\":\"tools/list\"}\n").await;
        let (_, mut stream) = peer.held().await;
        stream.write_all(response.as_bytes()).await.unwrap();
        drop(stream);
        assert_eq!(
            bridge.reply().await["id"],
            2,
            "failed notification stays silent"
        );
        // The early operation is refused locally; it never reached the peer.
        assert!(peer.exchanges.try_recv().is_err());
    }
    bridge
        .send("{\"method\":\"notifications/initialized\"}\n")
        .await;
    let (body, stream) = peer.held().await;
    release(&body, stream).await;
    bridge.send("[{\"method\":\"tools/call\"}]\n").await;
    let reply = bridge.reply().await;
    assert_eq!(
        reply["error"]["message"],
        "MCP 2025-11-25 does not support batches"
    );
    bridge
        .send(&format!(
            "[{{\"method\":\"tools/call\",\"pad\":\"{}\"}}]\n",
            "x".repeat(2_097_152)
        ))
        .await;
    let oversized = bridge.reply().await;
    assert_eq!(oversized["id"], serde_json::Value::Null);
    assert!(
        oversized["error"]["message"]
            .as_str()
            .unwrap()
            .contains("request body refused")
    );
    bridge.send(&ping_line("7")).await;
    let (body, stream) = peer.held().await;
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&body).unwrap()["id"],
        7
    );
    release(&body, stream).await;
    assert_eq!(bridge.reply().await["id"], 7);
    assert!(peer.exchanges.try_recv().is_err());
    let (code, rest) = bridge.finish().await;
    assert_eq!(code, 0);
    assert!(rest.is_empty());
}

#[tokio::test]
async fn incomplete_initialize_results_cannot_enable_operations() {
    use serde_json::json;
    for version in ["2025-03-26", "2025-06-18", "2025-11-25"] {
        let valid = json!({"protocolVersion":version,"capabilities":{"tools":{}},"serverInfo":{"name":"peer","version":"1"}});
        let mut invalid = Vec::new();
        for field in ["capabilities", "serverInfo"] {
            let mut result = valid.clone();
            result.as_object_mut().unwrap().remove(field);
            invalid.push(result);
            for value in [json!(null), json!(true), json!([]), json!("bad")] {
                let mut result = valid.clone();
                result[field] = value;
                invalid.push(result);
            }
        }
        for field in ["name", "version"] {
            let mut result = valid.clone();
            result["serverInfo"].as_object_mut().unwrap().remove(field);
            invalid.push(result);
            let mut result = valid.clone();
            result["serverInfo"][field] = json!(42);
            invalid.push(result);
        }
        for result in invalid {
            let mut peer = HeldPeer::start().await;
            let mut bridge = Running::spawn(format!("http://{}", peer.addr));
            // A modern proposal answered with malformed March metadata must
            // remain failed too: otherwise queued effects escape as legacy.
            bridge
                .send(&format!(
                    "{}{{\"method\":\"notifications/initialized\"}}\n{{\"id\":2,\"method\":\"tools/list\"}}\n{}",
                    revision_init(1, "2025-11-25"),
                    ping_line("3")
                ))
                .await;
            let (_, mut stream) = peer.held().await;
            let reply = json!({"jsonrpc":"2.0","id":1,"result":result});
            return_after_write(&reply.to_string(), &mut stream).await;
            drop(stream);
            assert_eq!(bridge.reply().await, reply, "forwarding remains verbatim");
            let (body, stream) = peer.held().await;
            assert_eq!(
                serde_json::from_str::<serde_json::Value>(&body).unwrap()["id"],
                3,
                "causal next-exchange witness"
            );
            assert!(
                !peer
                    .headers
                    .lock()
                    .unwrap()
                    .last()
                    .unwrap()
                    .contains("MCP-Protocol-Version")
            );
            assert_eq!(
                bridge.reply().await["error"]["code"],
                -32600,
                "incomplete {version} result enabled operations"
            );
            release(&body, stream).await;
            assert_eq!(bridge.reply().await["id"], 3);
            let (code, rest) = bridge.finish().await;
            assert_eq!(code, 0);
            assert!(rest.is_empty());
        }
    }
}

#[tokio::test]
async fn lost_and_truncated_initialization_stay_failed_until_explicit_retry() {
    for response in ["", "HTTP/1.1 200 OK\r\nContent-Length: 999\r\n\r\n{}"] {
        let mut peer = HeldPeer::start().await;
        let mut bridge = Running::spawn(format!("http://{}", peer.addr));
        bridge
            .send(&format!(
                "{}{{\"id\":2,\"method\":\"tools/list\"}}\n{}",
                revision_init(1, "2025-11-25"),
                ping_line("3")
            ))
            .await;
        let (_, mut stream) = peer.held().await;
        stream.write_all(response.as_bytes()).await.unwrap();
        drop(stream);
        let failure = bridge.reply().await;
        assert_eq!(failure["id"], 1);
        assert_eq!(failure["error"]["code"], -32000);
        assert_eq!(bridge.reply().await["error"]["code"], -32600);
        let (body, stream) = peer.held().await;
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&body).unwrap()["id"],
            3
        );
        release(&body, stream).await;
        assert_eq!(bridge.reply().await["id"], 3);
        bridge.send(&revision_init(4, "2025-11-25")).await;
        let (_, mut stream) = peer.held().await;
        revision_result(&mut stream, 4, "2025-11-25").await;
        drop(stream);
        assert_eq!(bridge.reply().await["id"], 4);
        bridge.send("{\"method\":\"notifications/initialized\"}\n{\"id\":5,\"method\":\"tools/list\"}\n").await;
        let (body, stream) = peer.held().await;
        release(&body, stream).await;
        let (body, stream) = peer.held().await;
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&body).unwrap()["id"],
            5
        );
        release(&body, stream).await;
        assert_eq!(bridge.reply().await["id"], 5);
        let (code, rest) = bridge.finish().await;
        assert_eq!(code, 0);
        assert!(rest.is_empty());
    }
}
