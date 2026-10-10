//! The stdio JSON-RPC bridge, driven in-process over duplex pipes.

use std::time::Duration;
use tokio::io::{AsyncBufReadExt as _, AsyncReadExt as _, AsyncWriteExt as _, duplex};

/// Drive the bridge with two duplex pipes and collect its stdout.
/// Input is fed and closed (EOF), the bridge's writes are read back.
async fn bridge_over(base_url: &str, input: &[u8]) -> (i32, String) {
    // Pipe 1: test writes input; the bridge reads it (until EOF).
    let (mut input_end, bridge_input) = duplex(64 * 1024);
    // Pipe 2: the bridge writes; the test reads.
    let (mut bridge_output, mut output_end) = duplex(64 * 1024);

    input_end.write_all(input).await.unwrap();
    drop(input_end); // EOF for the bridge's reader

    let (r, w) = tokio::io::split(bridge_input);
    drop(w); // the bridge's end is read-only
    let mut lines = tokio::io::BufReader::new(r).lines();
    let code = swarmail::stdio::bridge(base_url, &mut lines, &mut bridge_output).await;
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
    let input = b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{}}\n\n";
    let (code, out) = bridge_over(&base, input).await;
    assert_eq!(code, 0);
    let responses: Vec<serde_json::Value> = out
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(
        responses.len(),
        1,
        "blank line skipped, one response: {out}"
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
    let code = swarmail::stdio::bridge("http://127.0.0.1:1", &mut lines, &mut sink).await;
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
/// the bridge keeps serving the next line (one line in, two lines out is not
/// the contract — here a single request yields a single error reply).
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

/// A base that accepts the TCP connect but never answers: the read deadline
/// fires and the bridge answers with the read-timeout error.
#[tokio::test]
async fn silent_server_answers_as_a_read_timeout() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        // Accept and HOLD the stream, never reading or writing: the request
        // starves until the bridge's read deadline fires.
        let (_held, _) = listener.accept().await.unwrap();
        tokio::time::sleep(Duration::from_secs(30)).await;
    });

    let (code, out) = bridge_over(
        &format!("http://{addr}"),
        br#"{"jsonrpc":"2.0","id":11,"method":"initialize","params":{}}"#,
    )
    .await;
    assert_eq!(code, 0);
    let reply: serde_json::Value = serde_json::from_str(out.trim()).expect("one reply line");
    assert_eq!(reply["error"]["code"], -32000);
    assert_eq!(reply["error"]["message"], "read timeout", "{reply}");
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
